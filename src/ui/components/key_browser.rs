use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use super::Component;
use crate::action::Action;
use crate::types::KeyMetadata;

pub struct KeyBrowserProps<'a> {
    pub keys: &'a [Option<KeyMetadata>],
    pub total_count: Option<u64>,
    pub is_loading: bool,
    pub active_search_query: Option<&'a str>,
    pub is_active: bool,
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
        // Skip expensive operations if not active (optimization)
        // Still need to render for TUI, but can skip heavy calculations
        let is_active = props.is_active;

        // Calculate viewport height (subtract borders and title)
        self.viewport_height = area.height.saturating_sub(2) as usize;

        // Virtualized: build only the visible window around the selected index
        let total_count = props
            .total_count
            .map(|t| t as usize)
            .unwrap_or_else(|| props.keys.len());

        let selected_abs = self
            .state
            .selected()
            .unwrap_or(0)
            .min(total_count.saturating_sub(1));

        // Only compute view window if active or if selection changed (optimization)
        let (start_index, visible_len) = if is_active {
            self.compute_view_window(selected_abs, total_count)
        } else {
            // When inactive, use cached scroll_top or simple calculation
            let visible_len = self.viewport_height.min(total_count.max(1));
            let max_start = total_count.saturating_sub(visible_len);
            let start = self.scroll_top.min(max_start);
            (start, visible_len)
        };

        let content_width = area.width.saturating_sub(4) as usize;
        let mut key_items: Vec<ListItem> = Vec::with_capacity(visible_len);
        for offset in 0..visible_len {
            let abs = start_index + offset;
            if let Some(Some(k)) = props.keys.get(abs) {
                key_items.push(Self::build_key_item(k, content_width));
            } else {
                key_items.push(ListItem::new(Span::styled(
                    "...",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        // Build title
        let mut title = if let Some(query) = props.active_search_query {
            format!("Keys [Filter: '{}']", query)
        } else if let Some(total) = props.total_count {
            format!("Keys [{}]", total)
        } else {
            format!("Keys [{}]", total_count)
        };

        // Add position info
        let position = self
            .state
            .selected()
            .map(|i| format!(" [{} / {}]", i.saturating_add(1), total_count))
            .unwrap_or_else(|| " [—]".to_string());

        title.push_str(&position);

        // Add loading indicator if loading
        if props.is_loading {
            title.push_str(" [Loading...]");
        }

        let keys_list = List::new(key_items)
            .block(Self::card_block(title, props.is_active))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 34))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        // Use a local state with relative selection for the visible window
        let mut view_state = self.state.clone();
        if total_count > 0 {
            let rel = selected_abs.saturating_sub(start_index);
            view_state.select(Some(rel));
        } else {
            view_state.select(None);
        }
        f.render_stateful_widget(keys_list, area, &mut view_state);

        // Render scrollbar - represents full list
        if total_count > 0 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let scrollbar_area = area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            });

            let current_position = selected_abs;

            self.scrollbar_state = self
                .scrollbar_state
                .content_length(total_count)
                .position(current_position);

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scrollbar_state);
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
    /// Logic copied from AppState to keep component self-contained for rendering/input decisions
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

    fn build_key_item(key: &KeyMetadata, content_width: usize) -> ListItem<'static> {
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

        let line = Line::from(vec![
            Span::raw(name_display),
            Span::raw(spacer),
            Span::styled(type_label, Style::default().fg(Color::DarkGray)),
        ]);

        ListItem::new(line)
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

    fn card_block(title: String, is_active: bool) -> Block<'static> {
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        Block::default()
            .title(Line::from(title))
            .title_alignment(Alignment::Left)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
    }
}
