use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, ScrollbarState},
    Frame,
};

use super::Component;
use crate::action::Action;
use crate::types::KeyMetadata;
use crate::ui::theme::{self, AnimationState};

pub struct KeyBrowserProps<'a> {
    pub keys: &'a [Option<KeyMetadata>],
    pub total_count: Option<u64>,
    pub is_loading: bool,
    pub active_search_query: Option<&'a str>,
    pub is_active: bool,
    pub backend_type: Option<crate::types::BackendType>,
    pub is_searching: bool,
    /// Indices of keys matching the local fuzzy search (indices into keys array)
    pub search_results_local: &'a [usize],
    /// Keys from server search (may contain keys not in local list)
    pub search_results_server: &'a [KeyMetadata],
    /// Whether a server search is in progress
    pub is_server_searching: bool,
    /// Selection index within search results (0-based index into search_results_local)
    pub search_selection_index: Option<usize>,
    /// Animation state for visual effects
    pub animation: &'a AnimationState,
}

pub struct KeyBrowser {
    pub state: ListState,
    pub scrollbar_state: ScrollbarState,
    pub viewport_height: usize,
    scroll_top: usize,
}

impl KeyBrowser {
    const EDGE_SCROLL_GUARD: usize = 3;

    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            viewport_height: 20,
            scroll_top: 0,
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
}

impl Default for KeyBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for KeyBrowser {
    type Props<'a> = KeyBrowserProps<'a>;
    type Msg = Action;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let is_active = props.is_active;

        // Check if we're in search mode (has query)
        let has_search = props
            .active_search_query
            .map(|q| !q.is_empty())
            .unwrap_or(false);

        // Build title parts (left title, right info)
        let (title_left, title_right) = self.build_title_parts(&props, has_search, 0);

        // Render full-width header and get content area
        let content_area =
            theme::render_panel_header_split(f, area, &title_left, &title_right, is_active);

        self.viewport_height = content_area.height.saturating_sub(1) as usize;
        // Subtract: 3 for highlight symbol " ▸ " + 2 for scrollbar and padding
        let content_width = content_area.width.saturating_sub(5) as usize;

        // Build the display list - either filtered or full
        // Returns: (items, total_count, relative_selection_in_view, absolute_selection_for_scrollbar)
        let (key_items, display_count, selected_display_idx, scrollbar_position) = if has_search {
            // SEARCH MODE: Show only matching keys
            let (items, count, sel) = self.build_search_results_list(&props, content_width);
            (items, count, sel, sel.unwrap_or(0))
        } else {
            // NORMAL MODE: Show full key list with virtualization
            let (items, count, sel) = self.build_normal_list(&props, content_width, is_active);
            // For scrollbar, use the absolute position (self.state.selected), not the relative view position
            let abs_pos = self.state.selected().unwrap_or(0);
            (items, count, sel, abs_pos)
        };

        let keys_list = List::new(key_items)
            .highlight_style(theme::list_selected().add_modifier(Modifier::BOLD))
            .highlight_symbol(" ▸ ");

        let mut view_state = ListState::default();
        view_state.select(selected_display_idx);
        f.render_stateful_widget(keys_list, content_area, &mut view_state);

        // Render scrollbar with themed styling
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

    fn handle_input(&mut self, key: KeyEvent, props: Self::Props<'_>) -> Option<Self::Msg> {
        let total_count = props
            .total_count
            .map(|t| t as usize)
            .unwrap_or_else(|| props.keys.len());

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if total_count == 0 {
                    return None;
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

                // Check if we need to load more
                if self.needs_loading_around(i, props.keys, total_count) {
                    return Some(Action::LoadMoreKeys(i));
                }

                Some(Action::SelectKey(i))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total_count == 0 {
                    return None;
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

                // Check if we need to load more
                if self.needs_loading_around(i, props.keys, total_count) {
                    return Some(Action::LoadMoreKeys(i));
                }

                Some(Action::SelectKey(i))
            }
            _ => None,
        }
    }
}

impl KeyBrowser {
    /// Build the list for search mode - shows only matching keys
    fn build_search_results_list(
        &self,
        props: &KeyBrowserProps<'_>,
        content_width: usize,
    ) -> (Vec<ListItem<'static>>, usize, Option<usize>) {
        let mut items: Vec<ListItem<'static>> = Vec::new();

        // First add local matches (keys we already have loaded)
        for &key_idx in props.search_results_local.iter() {
            if let Some(Some(key)) = props.keys.get(key_idx) {
                items.push(Self::build_key_item(key, content_width, props.backend_type));
            }
        }

        // Then add server results that aren't already in local results
        let local_key_names: std::collections::HashSet<&str> = props
            .search_results_local
            .iter()
            .filter_map(|&idx| {
                props
                    .keys
                    .get(idx)
                    .and_then(|k| k.as_ref())
                    .map(|k| k.name.as_str())
            })
            .collect();

        for server_key in props.search_results_server.iter() {
            if !local_key_names.contains(server_key.name.as_str()) {
                items.push(Self::build_key_item_with_indicator(
                    server_key,
                    content_width,
                    props.backend_type,
                    "◦", // indicator that this is from server search
                ));
            }
        }

        // If no results, show a message
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

        // Use the dedicated search selection index
        (
            items,
            result_count.max(items_len),
            props.search_selection_index,
        )
    }

    /// Build the list for normal mode - full virtualized list
    fn build_normal_list(
        &mut self,
        props: &KeyBrowserProps<'_>,
        content_width: usize,
        is_active: bool,
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
                key_items.push(Self::build_key_item(k, content_width, props.backend_type));
            } else {
                // Loading placeholder with themed styling
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

    /// Build the title based on current mode
    /// Returns (left_title, right_info)
    fn build_title_parts(
        &self,
        props: &KeyBrowserProps<'_>,
        has_search: bool,
        display_count: usize,
    ) -> (String, String) {
        if props.is_searching {
            // Active search input mode
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
            // Has search filter but not actively typing
            if let Some(query) = props.active_search_query {
                (
                    format!("Keys │ /{}", query),
                    format!("{} results", display_count),
                )
            } else {
                ("Keys".to_string(), String::new())
            }
        } else {
            // Normal mode
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

    /// Format a number with thousand separators (e.g., 1,414)
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

    /// Check if we need to load the region around a specific index
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

        // Check N rows above and N rows below current position
        let start = index.saturating_sub(n);
        let end = (index + n).min(total);

        for i in start..end {
            if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                return true;
            }
        }

        // Wrap-around: if near start, check last N rows
        if index < n {
            let wrap_start = total.saturating_sub(n);
            for i in wrap_start..total {
                if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        // Wrap-around: if near end, check first N rows
        if index + n >= total {
            for i in 0..n.min(total) {
                if keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        false
    }

    fn build_key_item(
        key: &KeyMetadata,
        content_width: usize,
        backend_type: Option<crate::types::BackendType>,
    ) -> ListItem<'static> {
        // Don't show key type for memcached or etcd (they don't have typed keys)
        let show_type = !matches!(
            backend_type,
            Some(crate::types::BackendType::Memcached | crate::types::BackendType::Etcd)
        );

        if show_type {
            let type_label = key.value_type.to_string();
            let type_width = type_label.chars().count();
            let min_gap = if content_width > type_width { 1 } else { 0 };
            let available_for_name = content_width.saturating_sub(type_width + min_gap);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width = content_width.saturating_sub(name_width + type_width);
            let spacer = if spacer_width > 0 {
                " ".repeat(spacer_width)
            } else {
                String::new()
            };

            // Type-specific colors
            let type_color = match key.value_type {
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
            };

            let line = Line::from(vec![
                Span::styled(name_display, Style::default().fg(theme::TEXT_PRIMARY())),
                Span::raw(spacer),
                Span::styled(
                    type_label,
                    Style::default().fg(type_color).add_modifier(Modifier::DIM),
                ),
            ]);

            ListItem::new(line)
        } else {
            // For memcached, just show the key name without type
            let name_display = Self::truncate_to_fit(&key.name, content_width);
            let line = Line::from(vec![Span::styled(
                name_display,
                Style::default().fg(theme::TEXT_PRIMARY()),
            )]);
            ListItem::new(line)
        }
    }

    /// Build a key item with a leading indicator (for server search results)
    fn build_key_item_with_indicator(
        key: &KeyMetadata,
        content_width: usize,
        backend_type: Option<crate::types::BackendType>,
        indicator: &str,
    ) -> ListItem<'static> {
        let show_type = !matches!(
            backend_type,
            Some(crate::types::BackendType::Memcached | crate::types::BackendType::Etcd)
        );
        let indicator_width = indicator.chars().count() + 1; // +1 for space

        if show_type {
            let type_label = key.value_type.to_string();
            let type_width = type_label.chars().count();
            let min_gap = if content_width > type_width + indicator_width {
                1
            } else {
                0
            };
            let available_for_name =
                content_width.saturating_sub(type_width + min_gap + indicator_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let name_width = name_display.chars().count();
            let spacer_width =
                content_width.saturating_sub(name_width + type_width + indicator_width);
            let spacer = if spacer_width > 0 {
                " ".repeat(spacer_width)
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(theme::NEON_AMBER()),
                ),
                Span::styled(name_display, Style::default().fg(theme::TEXT_PRIMARY())),
                Span::raw(spacer),
                Span::styled(type_label, Style::default().fg(theme::TEXT_DIM())),
            ]);

            ListItem::new(line)
        } else {
            let available_for_name = content_width.saturating_sub(indicator_width);
            let name_display = Self::truncate_to_fit(&key.name, available_for_name);
            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(theme::NEON_AMBER()),
                ),
                Span::styled(name_display, Style::default().fg(theme::TEXT_PRIMARY())),
            ]);
            ListItem::new(line)
        }
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
}
