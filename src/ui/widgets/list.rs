//! SelectableList widget - a list with selection and scrolling
//!
//! This is a higher-level abstraction over ratatui's List widget
//! with built-in selection state management.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState, ScrollbarState},
    Frame,
};

use crate::ui::theme;

/// State for a selectable list
#[derive(Debug, Default)]
pub struct SelectableListState {
    /// Internal ratatui list state
    pub list_state: ListState,
    /// Scrollbar state
    pub scrollbar_state: ScrollbarState,
    /// Current scroll offset for virtualization
    pub scroll_offset: usize,
    /// Viewport height (visible rows)
    pub viewport_height: usize,
    /// Total number of items
    pub total_items: usize,
}

impl SelectableListState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Select an index
    pub fn select(&mut self, index: Option<usize>) {
        self.list_state.select(index);
        if index.is_none() {
            self.scroll_offset = 0;
        }
    }

    /// Move selection to next item (with wraparound)
    pub fn next(&mut self) {
        if self.total_items == 0 {
            return;
        }
        let current = self.selected().unwrap_or(0);
        let next = if current >= self.total_items.saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.select(Some(next));
    }

    /// Move selection to previous item (with wraparound)
    pub fn prev(&mut self) {
        if self.total_items == 0 {
            return;
        }
        let current = self.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.total_items.saturating_sub(1)
        } else {
            current - 1
        };
        self.select(Some(prev));
    }

    /// Update viewport dimensions
    pub fn set_viewport(&mut self, height: usize, total: usize) {
        self.viewport_height = height;
        self.total_items = total;
    }

    /// Get the visible range (start_index, count) for virtualization
    pub fn visible_range(&self) -> (usize, usize) {
        let visible_count = self.viewport_height.min(self.total_items);
        let max_offset = self.total_items.saturating_sub(visible_count);
        let offset = self.scroll_offset.min(max_offset);
        (offset, visible_count)
    }

    /// Update scroll offset to keep selection visible
    pub fn ensure_visible(&mut self) {
        if let Some(selected) = self.selected() {
            let visible_count = self.viewport_height.min(self.total_items);
            if visible_count == 0 {
                return;
            }

            // Scroll up if selection is above viewport
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            }
            // Scroll down if selection is below viewport
            else if selected >= self.scroll_offset + visible_count {
                self.scroll_offset = selected.saturating_sub(visible_count.saturating_sub(1));
            }
        }
    }

    /// Get selection index relative to current scroll offset
    pub fn relative_selection(&self) -> Option<usize> {
        self.selected()
            .map(|s| s.saturating_sub(self.scroll_offset))
    }
}

/// A list widget with selection support
pub struct SelectableList<'a> {
    items: Vec<ListItem<'a>>,
    highlight_symbol: &'a str,
    is_active: bool,
}

impl<'a> SelectableList<'a> {
    pub fn new(items: Vec<ListItem<'a>>) -> Self {
        Self {
            items,
            highlight_symbol: " ▸ ",
            is_active: false,
        }
    }

    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Render the list with the given state
    pub fn render(self, f: &mut Frame, area: Rect, state: &mut SelectableListState) {
        // Update viewport
        state.set_viewport(area.height as usize, self.items.len());
        state.ensure_visible();

        let highlight_style = if self.is_active {
            theme::list_selected().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let list = List::new(self.items)
            .highlight_style(highlight_style)
            .highlight_symbol(self.highlight_symbol);

        // Create a view state with relative selection
        let mut view_state = ListState::default();
        view_state.select(state.relative_selection());

        f.render_stateful_widget(list, area, &mut view_state);

        // Render scrollbar if needed
        if state.total_items > state.viewport_height && area.width > 2 {
            state.scrollbar_state = state
                .scrollbar_state
                .content_length(state.total_items)
                .viewport_content_length(state.viewport_height)
                .position(state.selected().unwrap_or(0));

            f.render_stateful_widget(
                theme::scrollbar(self.is_active),
                theme::scrollbar_area(area),
                &mut state.scrollbar_state,
            );
        }
    }
}
