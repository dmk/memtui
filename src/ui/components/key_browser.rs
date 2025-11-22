use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{
        Block, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
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
}

impl KeyBrowser {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            viewport_height: 20,
        }
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn viewport_height(&self) -> usize {
        self.viewport_height
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
        let visible_len = self.viewport_height.min(total_count.max(1));

        // Calculate viewport start to keep selection visible and scroll smoothly
        // Strategy: keep selection at a fixed position in the viewport (1/2 from top)
        let scroll_offset = visible_len / 2;
        let max_start = total_count.saturating_sub(visible_len);

        let start_index = if total_count == 0 || visible_len == 0 {
            0
        } else {
            // Calculate start_index so that selected_abs appears at scroll_offset position
            let ideal_start = selected_abs.saturating_sub(scroll_offset);
            // Clamp to valid range [0, max_start]
            ideal_start.min(max_start).max(0)
        };

        let mut key_items: Vec<ListItem> = Vec::with_capacity(visible_len);
        for offset in 0..visible_len {
            let abs = start_index + offset;
            if let Some(Some(k)) = props.keys.get(abs) {
                key_items.push(ListItem::new(k.name.as_str()));
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
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if props.is_active {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

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
}
