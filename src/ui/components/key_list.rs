//! KeyList component - displays the list of keys with search support
//!
//! This component subscribes to Key, Scroll, and Global events.
//! It manages its own UI state (selection, scroll position).

use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, ScrollbarState},
    Frame,
};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Clone, Copy)]
enum TtlUrgency {
    Normal,
    Soon,
    Imminent,
}

use super::Component;
use crate::action::Action;
use crate::backend::BackendCapabilities;
use crate::events::{ComponentId, Event, EventKind, EventType};
use crate::types::KeyMetadata;
use crate::ui::theme::{self, AnimationState};

/// Props for KeyList rendering
pub struct KeyListProps<'a> {
    pub keys: &'a [Option<KeyMetadata>],
    pub total_count: Option<u64>,
    pub is_loading: bool,
    pub active_search_query: Option<&'a str>,
    pub is_active: bool,
    pub capabilities: Option<&'a BackendCapabilities>,
    pub is_searching: bool,
    /// Indices of keys matching the local fuzzy search
    pub search_results_local: &'a [usize],
    /// Keys from server search (may contain keys not in local list)
    pub search_results_server: &'a [KeyMetadata],
    /// Whether a server search is in progress
    pub is_server_searching: bool,
    /// Selection index within search results
    pub search_selection_index: Option<usize>,
    /// Animation state for visual effects
    pub animation: &'a AnimationState,
    /// Match positions for highlighting
    pub search_match_positions: &'a HashMap<usize, Vec<u32>>,
    pub now: SystemTime,
}

/// KeyList component state
pub struct KeyList {
    /// List selection state
    pub state: ListState,
    /// Scrollbar state
    pub scrollbar_state: ScrollbarState,
    /// Viewport height (visible rows)
    pub viewport_height: usize,
    /// Scroll offset for virtualization
    scroll_top: usize,
    /// Last rendered area
    last_area: Option<Rect>,
}

impl KeyList {
    const EDGE_SCROLL_GUARD: usize = 3;
    const TTL_COL_WIDTH: usize = 6;
    const TYPE_COL_WIDTH: usize = 7;

    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            viewport_height: 20,
            scroll_top: 0,
            last_area: None,
        }
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
        if index.is_none() {
            self.scroll_top = 0;
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn viewport_height(&self) -> usize {
        self.viewport_height
    }

    pub fn view_bounds(&self, total_count: usize) -> Option<(usize, usize)> {
        if total_count == 0 {
            return None;
        }

        let visible_len = self.viewport_height.min(total_count.max(1));
        if visible_len == 0 {
            return None;
        }

        let max_start = total_count.saturating_sub(visible_len);
        let start_index = self.scroll_top.min(max_start);

        Some((start_index, visible_len))
    }

    /// Navigate to next item
    pub fn next(&mut self, total: usize) {
        if total == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let next = if current >= total.saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.state.select(Some(next));
    }

    /// Navigate to previous item
    pub fn prev(&mut self, total: usize) {
        if total == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let prev = if current == 0 {
            total.saturating_sub(1)
        } else {
            current - 1
        };
        self.state.select(Some(prev));
    }

    /// Get index from click position
    pub fn index_at_position(
        &self,
        area: Rect,
        column: u16,
        row: u16,
        total: usize,
    ) -> Option<usize> {
        if area.height <= 2 || area.width <= 2 {
            return None;
        }

        let inner_left = area.x.saturating_add(1);
        let inner_right = area.x.saturating_add(area.width.saturating_sub(1));
        if column < inner_left || column >= inner_right {
            return None;
        }

        let inner_top = area.y.saturating_add(1);
        let inner_bottom = area.y.saturating_add(area.height.saturating_sub(1));
        if row < inner_top || row >= inner_bottom {
            return None;
        }

        let (start_index, visible_len) = self.view_bounds(total)?;
        let rel = (row - inner_top) as usize;
        if rel >= visible_len {
            return None;
        }

        let index = start_index + rel;
        if index >= total {
            return None;
        }

        Some(index)
    }

    // =========================================================================
    // Rendering helpers
    // =========================================================================

    fn build_title_parts(
        &self,
        props: &KeyListProps<'_>,
        has_search: bool,
        display_count: usize,
    ) -> (String, String) {
        if props.is_searching {
            if let Some(query) = props.active_search_query {
                if query.is_empty() {
                    ("Keys │ ▍".to_string(), String::new())
                } else {
                    let server_indicator = if props.is_server_searching {
                        format!(" {}", theme::spinner(props.animation))
                    } else {
                        String::new()
                    };
                    (
                        format!("Keys │ {}▍{}", query, server_indicator),
                        format!("{} found", display_count),
                    )
                }
            } else {
                ("Keys │ ▍".to_string(), String::new())
            }
        } else if has_search {
            if let Some(query) = props.active_search_query {
                (
                    format!("Keys │ /{}", query),
                    format!("{} results", display_count),
                )
            } else {
                ("Keys".to_string(), String::new())
            }
        } else {
            let total_count = props
                .total_count
                .map(|t| t as usize)
                .unwrap_or(props.keys.len());

            let loading_indicator = if props.is_loading {
                format!(" {}", theme::spinner(props.animation))
            } else {
                String::new()
            };

            let right = if let Some(i) = self.state.selected() {
                format!(
                    "{} of {}{}",
                    Self::format_number(i.saturating_add(1)),
                    Self::format_number(total_count),
                    loading_indicator
                )
            } else {
                format!("{}{}", Self::format_number(total_count), loading_indicator)
            };

            ("Keys".to_string(), right)
        }
    }

    fn format_number(n: usize) -> String {
        let s = n.to_string();
        let mut result = String::with_capacity(s.len() + s.len() / 3);
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result.chars().rev().collect()
    }

    fn compute_view_window(&mut self, selected_abs: usize, total_count: usize) -> (usize, usize) {
        let visible_len = self.viewport_height.min(total_count.max(1));
        if visible_len == 0 {
            self.scroll_top = 0;
            return (0, 0);
        }

        let max_start = total_count.saturating_sub(visible_len);
        self.scroll_top = self.scroll_top.min(max_start);

        if selected_abs < self.scroll_top {
            self.scroll_top = selected_abs;
        } else {
            let last_visible = self
                .scroll_top
                .saturating_add(visible_len.saturating_sub(1));
            if selected_abs > last_visible {
                self.scroll_top = selected_abs.saturating_sub(visible_len.saturating_sub(1));
            }
        }

        let guard = Self::edge_guard_for(visible_len);
        if guard > 0 {
            let top_threshold = self.scroll_top.saturating_add(guard);
            let bottom_anchor = visible_len.saturating_sub(guard + 1);
            let bottom_threshold = self.scroll_top.saturating_add(bottom_anchor);

            if selected_abs < top_threshold {
                let new_top = selected_abs.saturating_sub(guard);
                self.scroll_top = new_top.min(max_start);
            } else if selected_abs > bottom_threshold {
                let new_top = selected_abs.saturating_sub(bottom_anchor);
                self.scroll_top = new_top.min(max_start);
            }
        }

        (self.scroll_top, visible_len)
    }

    fn edge_guard_for(visible_len: usize) -> usize {
        if visible_len <= 1 {
            return 0;
        }
        let max_guard = (visible_len - 1) / 2;
        Self::EDGE_SCROLL_GUARD.min(max_guard)
    }

    fn build_search_results_list(
        &self,
        props: &KeyListProps<'_>,
        content_width: usize,
        now: SystemTime,
    ) -> (Vec<ListItem<'static>>, usize, Option<usize>) {
        let mut items: Vec<ListItem<'static>> = Vec::new();

        // All search results (local + merged server) are now in search_results_local
        for &key_idx in props.search_results_local.iter() {
            if let Some(Some(key)) = props.keys.get(key_idx) {
                let match_positions = props.search_match_positions.get(&key_idx);
                items.push(Self::build_key_item_highlighted(
                    key,
                    content_width,
                    props.capabilities,
                    match_positions,
                    now,
                ));
            }
        }

        if items.is_empty() {
            if props.is_server_searching {
                let spinner = theme::spinner(props.animation);
                items.push(ListItem::new(Span::styled(
                    format!("{} Searching...", spinner),
                    Style::default()
                        .fg(theme::NEON_AMBER())
                        .add_modifier(Modifier::ITALIC),
                )));
            } else if props
                .active_search_query
                .map(|q| !q.is_empty())
                .unwrap_or(false)
            {
                items.push(ListItem::new(Span::styled(
                    "  No matches found",
                    Style::default()
                        .fg(theme::TEXT_DIM())
                        .add_modifier(Modifier::ITALIC),
                )));
            }
        }

        let items_len = items.len();
        let result_count = props.search_results_local.len();

        (
            items,
            result_count.max(items_len),
            props.search_selection_index,
        )
    }

    fn build_normal_list(
        &mut self,
        props: &KeyListProps<'_>,
        content_width: usize,
        is_active: bool,
        now: SystemTime,
    ) -> (Vec<ListItem<'static>>, usize, Option<usize>) {
        let total_count = props
            .total_count
            .map(|t| t as usize)
            .unwrap_or_else(|| props.keys.len());

        let selected_abs = self
            .state
            .selected()
            .unwrap_or(0)
            .min(total_count.saturating_sub(1));

        let (start_index, visible_len) = if is_active {
            self.compute_view_window(selected_abs, total_count)
        } else {
            let visible_len = self.viewport_height.min(total_count.max(1));
            let max_start = total_count.saturating_sub(visible_len);
            let start = self.scroll_top.min(max_start);
            (start, visible_len)
        };

        let mut key_items: Vec<ListItem<'static>> = Vec::with_capacity(visible_len);
        for offset in 0..visible_len {
            let abs = start_index + offset;
            if let Some(Some(k)) = props.keys.get(abs) {
                key_items.push(Self::build_key_item(
                    k,
                    content_width,
                    props.capabilities,
                    now,
                ));
            } else {
                let spinner = theme::spinner_dots(props.animation);
                key_items.push(ListItem::new(Span::styled(
                    format!("  {} loading...", spinner),
                    Style::default().fg(theme::TEXT_DIM()),
                )));
            }
        }

        let rel_selection = if total_count > 0 {
            Some(selected_abs.saturating_sub(start_index))
        } else {
            None
        };

        (key_items, total_count, rel_selection)
    }

    fn build_key_item(
        key: &KeyMetadata,
        content_width: usize,
        capabilities: Option<&BackendCapabilities>,
        now: SystemTime,
    ) -> ListItem<'static> {
        let show_type = capabilities
            .map(|c| c.supports_type_display)
            .unwrap_or(false);
        let show_ttl = capabilities
            .map(|c| c.supports_expiry_display)
            .unwrap_or(false);

        let ttl_meta = if show_ttl {
            Self::ttl_meta(now, key)
        } else {
            None
        };

        if show_type {
            let meta_width = if show_ttl {
                1 + Self::TTL_COL_WIDTH + 1 + Self::TYPE_COL_WIDTH
            } else {
                1 + Self::TYPE_COL_WIDTH
            };
            let available_for_name = content_width.saturating_sub(meta_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width = content_width.saturating_sub(name_width + meta_width);
            let spacer = " ".repeat(spacer_width);

            let type_color = Self::type_color(key.value_type);

            let mut spans = vec![
                Span::styled(name_display, Style::default().fg(theme::TEXT_PRIMARY())),
                Span::raw(spacer),
            ];
            spans.push(Span::raw(" "));
            if show_ttl {
                spans.push(Self::ttl_span(ttl_meta));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                Self::padded_type_label(key.value_type),
                Style::default().fg(type_color).add_modifier(Modifier::DIM),
            ));
            let line = Line::from(spans);

            ListItem::new(line)
        } else {
            let meta_width = if show_ttl { 1 + Self::TTL_COL_WIDTH } else { 0 };
            let available_for_name = content_width.saturating_sub(meta_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width = content_width.saturating_sub(name_width + meta_width);
            let spacer = " ".repeat(spacer_width);

            let mut spans = vec![
                Span::styled(name_display, Style::default().fg(theme::TEXT_PRIMARY())),
                Span::raw(spacer),
            ];
            if show_ttl {
                spans.push(Span::raw(" "));
                spans.push(Self::ttl_span(ttl_meta));
            }

            let line = Line::from(spans);
            ListItem::new(line)
        }
    }

    fn type_color(value_type: crate::types::ValueType) -> ratatui::style::Color {
        match value_type {
            crate::types::ValueType::String => theme::NEON_CYAN(),
            crate::types::ValueType::Hash => theme::NEON_PURPLE(),
            crate::types::ValueType::List => theme::NEON_GREEN(),
            crate::types::ValueType::Set => theme::NEON_AMBER(),
            crate::types::ValueType::SortedSet => theme::NEON_PINK(),
            crate::types::ValueType::Binary => theme::TEXT_SECONDARY(),
            crate::types::ValueType::Json => theme::ELECTRIC_BLUE(),
            crate::types::ValueType::Integer => theme::NEON_GREEN(),
            crate::types::ValueType::Float => theme::NEON_AMBER(),
            crate::types::ValueType::Unknown => theme::TEXT_DIM(),
        }
    }

    fn padded_type_label(value_type: crate::types::ValueType) -> String {
        let type_label = value_type.to_string();
        format!("{:>width$}", type_label, width = Self::TYPE_COL_WIDTH)
    }

    fn ttl_meta(now: SystemTime, key: &KeyMetadata) -> Option<(String, TtlUrgency)> {
        let expires_at = key.expires_at.or_else(|| {
            key.ttl.and_then(|ttl| {
                key.last_accessed
                    .and_then(|seen| seen.checked_add(ttl))
                    .or_else(|| now.checked_add(ttl))
            })
        });

        let expires_at = expires_at?;
        let remaining = match expires_at.duration_since(now) {
            Ok(duration) => duration,
            Err(_) => return Some(("exp".to_string(), TtlUrgency::Imminent)),
        };

        let urgency = if remaining <= Duration::from_secs(60) {
            TtlUrgency::Imminent
        } else if remaining <= Duration::from_secs(300) {
            TtlUrgency::Soon
        } else {
            TtlUrgency::Normal
        };

        if remaining < Duration::from_secs(1) {
            return Some(("0s".to_string(), urgency));
        }

        Some((Self::format_duration_compact(remaining), urgency))
    }

    fn ttl_span(ttl_meta: Option<(String, TtlUrgency)>) -> Span<'static> {
        let (label, urgency) = ttl_meta
            .map(|(ttl, urgency)| (Self::pad_ttl(ttl), urgency))
            .unwrap_or_else(|| (" ".repeat(Self::TTL_COL_WIDTH), TtlUrgency::Normal));

        let mut style = Style::default().add_modifier(Modifier::DIM);
        style = match urgency {
            TtlUrgency::Imminent => style
                .fg(theme::NEON_RED())
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
            TtlUrgency::Soon => style.fg(theme::NEON_AMBER()).add_modifier(Modifier::BOLD),
            TtlUrgency::Normal => style.fg(theme::TEXT_DIM()),
        };

        Span::styled(label, style)
    }

    fn pad_ttl(ttl: String) -> String {
        let mut label = ttl;
        if label.chars().count() > Self::TTL_COL_WIDTH {
            label = label.chars().take(Self::TTL_COL_WIDTH).collect();
        }
        format!("{:>width$}", label, width = Self::TTL_COL_WIDTH)
    }

    fn format_duration_compact(duration: Duration) -> String {
        let secs = duration.as_secs();
        let days = secs / 86_400;
        if days >= 10_000 {
            return "9999d".to_string();
        }
        if days > 0 {
            if days >= 10 {
                return format!("{}d", days);
            }
            let hours = (secs % 86_400) / 3_600;
            if hours == 0 {
                return format!("{}d", days);
            }
            return format!("{}d {}h", days, hours);
        }

        let hours = secs / 3_600;
        if hours > 0 {
            if hours >= 10 {
                return format!("{}h", hours);
            }
            let minutes = (secs % 3_600) / 60;
            if minutes == 0 {
                return format!("{}h", hours);
            }
            return format!("{}h {}m", hours, minutes);
        }

        let minutes = secs / 60;
        if minutes > 0 {
            if minutes >= 10 {
                return format!("{}m", minutes);
            }
            let seconds = secs % 60;
            if seconds == 0 {
                return format!("{}m", minutes);
            }
            return format!("{}m {}s", minutes, seconds);
        }

        format!("{}s", secs)
    }

    fn truncate_to_fit(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }

        let char_count = text.chars().count();
        if char_count <= max_chars {
            return text.to_string();
        }

        if max_chars <= 3 {
            return text.chars().take(max_chars).collect();
        }

        let mut truncated: String = text.chars().take(max_chars - 3).collect();
        truncated.push_str("...");
        truncated
    }

    fn build_key_item_highlighted(
        key: &KeyMetadata,
        content_width: usize,
        capabilities: Option<&BackendCapabilities>,
        match_positions: Option<&Vec<u32>>,
        now: SystemTime,
    ) -> ListItem<'static> {
        let show_type = capabilities
            .map(|c| c.supports_type_display)
            .unwrap_or(false);
        let show_ttl = capabilities
            .map(|c| c.supports_expiry_display)
            .unwrap_or(false);

        let ttl_meta = if show_ttl {
            Self::ttl_meta(now, key)
        } else {
            None
        };

        if show_type {
            let meta_width = if show_ttl {
                1 + Self::TTL_COL_WIDTH + 1 + Self::TYPE_COL_WIDTH
            } else {
                1 + Self::TYPE_COL_WIDTH
            };
            let available_for_name = content_width.saturating_sub(meta_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width = content_width.saturating_sub(name_width + meta_width);
            let spacer = " ".repeat(spacer_width);

            let type_color = Self::type_color(key.value_type);
            let name_spans = Self::build_highlighted_spans(&name_display, match_positions);

            let mut spans = name_spans;
            spans.push(Span::raw(spacer));
            spans.push(Span::raw(" "));
            if show_ttl {
                spans.push(Self::ttl_span(ttl_meta));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                Self::padded_type_label(key.value_type),
                Style::default().fg(type_color).add_modifier(Modifier::DIM),
            ));

            ListItem::new(Line::from(spans))
        } else {
            let meta_width = if show_ttl { 1 + Self::TTL_COL_WIDTH } else { 0 };
            let available_for_name = content_width.saturating_sub(meta_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width = content_width.saturating_sub(name_width + meta_width);
            let spacer = " ".repeat(spacer_width);
            let name_spans = Self::build_highlighted_spans(&name_display, match_positions);
            let mut spans = name_spans;
            spans.push(Span::raw(spacer));
            if show_ttl {
                spans.push(Span::raw(" "));
                spans.push(Self::ttl_span(ttl_meta));
            }
            ListItem::new(Line::from(spans))
        }
    }

    fn build_highlighted_spans(
        text: &str,
        match_positions: Option<&Vec<u32>>,
    ) -> Vec<Span<'static>> {
        let positions: std::collections::HashSet<u32> = match_positions
            .map(|p| p.iter().copied().collect())
            .unwrap_or_default();

        if positions.is_empty() {
            return vec![Span::styled(
                text.to_string(),
                Style::default().fg(theme::TEXT_PRIMARY()),
            )];
        }

        let mut spans = Vec::new();
        let mut current_segment = String::new();
        let mut current_is_highlighted = false;

        for (idx, ch) in text.chars().enumerate() {
            let is_match = positions.contains(&(idx as u32));

            if is_match != current_is_highlighted && !current_segment.is_empty() {
                let style = if current_is_highlighted {
                    Style::default()
                        .fg(theme::NEON_AMBER())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_PRIMARY())
                };
                spans.push(Span::styled(current_segment.clone(), style));
                current_segment.clear();
            }

            current_segment.push(ch);
            current_is_highlighted = is_match;
        }

        if !current_segment.is_empty() {
            let style = if current_is_highlighted {
                Style::default()
                    .fg(theme::NEON_AMBER())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_PRIMARY())
            };
            spans.push(Span::styled(current_segment, style));
        }

        spans
    }

    fn needs_loading_around(
        &self,
        index: usize,
        keys: &[Option<KeyMetadata>],
        total: usize,
    ) -> bool {
        let n = self.viewport_height;

        if total == 0 {
            return false;
        }

        let start = index.saturating_sub(n);
        let end = (index + n).min(total);

        for i in start..end {
            if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                return true;
            }
        }

        if index < n {
            let wrap_start = total.saturating_sub(n);
            for i in wrap_start..total {
                if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        if index + n >= total {
            for i in 0..n.min(total) {
                if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for KeyList {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for KeyList {
    type Props<'a> = KeyListProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key, EventType::Scroll, EventType::Global]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        match &event.kind {
            EventKind::Key(key) => {
                // Handle global events regardless of focus
                if key.code == KeyCode::Esc && !props.active_search_query.unwrap_or("").is_empty() {
                    return vec![Action::ClearSearch];
                }

                if event.context.focused_component == Some(ComponentId::KeyList) {
                    self.handle_key_event(key, &props)
                } else {
                    vec![]
                }
            }
            EventKind::Scroll { column, row, delta } => {
                if event
                    .context
                    .point_in_component(ComponentId::KeyList, *column, *row)
                {
                    self.handle_scroll_event(*delta, &props)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        self.last_area = Some(area);
        let is_active = props.is_active;
        let now = props.now;

        let has_search = props
            .active_search_query
            .map(|q| !q.is_empty())
            .unwrap_or(false);

        let search_result_count = if has_search {
            props.search_results_local.len()
        } else {
            0
        };

        let (title_left, title_right) =
            self.build_title_parts(&props, has_search, search_result_count);

        let content_area =
            theme::render_panel_header_split(f, area, &title_left, &title_right, is_active);

        self.viewport_height = content_area.height.saturating_sub(1) as usize;
        let content_width = content_area.width.saturating_sub(5) as usize;

        let (key_items, display_count, selected_display_idx, scrollbar_position) = if has_search {
            let (items, count, sel) = self.build_search_results_list(&props, content_width, now);
            (items, count, sel, sel.unwrap_or(0))
        } else {
            let (items, count, sel) = self.build_normal_list(&props, content_width, is_active, now);
            let abs_pos = self.state.selected().unwrap_or(0);
            (items, count, sel, abs_pos)
        };

        let keys_list = List::new(key_items)
            .highlight_style(theme::list_selected().add_modifier(Modifier::BOLD))
            .highlight_symbol(" ▸ ");

        let mut view_state = ListState::default();
        view_state.select(selected_display_idx);
        f.render_stateful_widget(keys_list, content_area, &mut view_state);

        if display_count > 0 && content_area.width > 2 {
            self.scrollbar_state = self
                .scrollbar_state
                .content_length(display_count)
                .position(scrollbar_position);

            f.render_stateful_widget(
                theme::scrollbar(is_active),
                theme::scrollbar_area(content_area),
                &mut self.scrollbar_state,
            );
        }
    }

    fn area(&self) -> Option<Rect> {
        self.last_area
    }
}

impl KeyList {
    fn handle_key_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        props: &KeyListProps<'_>,
    ) -> Vec<Action> {
        // When search is active, navigate through search results only
        let has_active_search = !props.active_search_query.unwrap_or("").is_empty();

        if has_active_search {
            return self.handle_search_key_event(key, props);
        }

        let total_count = props
            .total_count
            .map(|t| t as usize)
            .unwrap_or_else(|| props.keys.len());

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if total_count == 0 {
                    return vec![];
                }
                let i = match self.state.selected() {
                    Some(i) => {
                        if i >= total_count - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));

                let mut actions = vec![Action::SelectKey(i)];
                if self.needs_loading_around(i, props.keys, total_count) {
                    actions.push(Action::LoadMoreKeys(i));
                }
                actions
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total_count == 0 {
                    return vec![];
                }
                let i = match self.state.selected() {
                    Some(i) => {
                        if i == 0 {
                            total_count - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));

                let mut actions = vec![Action::SelectKey(i)];
                if self.needs_loading_around(i, props.keys, total_count) {
                    actions.push(Action::LoadMoreKeys(i));
                }
                actions
            }
            _ => vec![],
        }
    }

    fn handle_search_key_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        props: &KeyListProps<'_>,
    ) -> Vec<Action> {
        let result_count = props.search_results_local.len();
        if result_count == 0 {
            return vec![];
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let current_search_idx = props.search_selection_index.unwrap_or(0);
                let new_search_idx = if current_search_idx >= result_count - 1 {
                    0
                } else {
                    current_search_idx + 1
                };

                if let Some(&key_idx) = props.search_results_local.get(new_search_idx) {
                    self.state.select(Some(key_idx));
                    vec![Action::SearchNextResult]
                } else {
                    vec![]
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let current_search_idx = props.search_selection_index.unwrap_or(0);
                let new_search_idx = if current_search_idx == 0 {
                    result_count - 1
                } else {
                    current_search_idx - 1
                };

                if let Some(&key_idx) = props.search_results_local.get(new_search_idx) {
                    self.state.select(Some(key_idx));
                    vec![Action::SearchPrevResult]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn handle_scroll_event(&mut self, delta: isize, props: &KeyListProps<'_>) -> Vec<Action> {
        let total_count = props
            .total_count
            .map(|t| t as usize)
            .unwrap_or_else(|| props.keys.len());

        if total_count == 0 {
            return vec![];
        }

        let current = self.state.selected().unwrap_or(0);
        let new_index = if delta > 0 {
            // Scroll down
            let next = current.saturating_add(delta.unsigned_abs());
            if next >= total_count {
                0
            } else {
                next
            }
        } else {
            // Scroll up
            let amount = delta.unsigned_abs();
            if amount > current {
                total_count.saturating_sub(1)
            } else {
                current - amount
            }
        };

        if new_index != current {
            self.state.select(Some(new_index));
            let mut actions = vec![Action::SelectKey(new_index)];
            if self.needs_loading_around(new_index, props.keys, total_count) {
                actions.push(Action::LoadMoreKeys(new_index));
            }
            actions
        } else {
            vec![]
        }
    }
}
