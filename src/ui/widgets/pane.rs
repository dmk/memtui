//! Pane widget - bordered container with title and optional focus state
//!
//! This is the standard container used for panels like KeyList and ValueViewer.

use ratatui::{layout::Rect, widgets::ScrollbarState, Frame};

use crate::ui::theme;

/// Style configuration for a Pane
#[derive(Debug, Clone, Default)]
pub struct PaneStyle {
    /// Whether the pane is focused/active
    pub is_active: bool,
    /// Show border
    pub show_border: bool,
    /// Show scrollbar
    pub show_scrollbar: bool,
}

impl PaneStyle {
    pub fn new() -> Self {
        Self {
            is_active: false,
            show_border: true,
            show_scrollbar: true,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

/// A bordered container with a title and optional scrollbar
pub struct Pane<'a> {
    /// Left side of the title
    title_left: &'a str,
    /// Right side of the title (e.g., "1 of 100")
    title_right: Option<&'a str>,
    /// Style configuration
    style: PaneStyle,
}

impl<'a> Pane<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title_left: title,
            title_right: None,
            style: PaneStyle::new(),
        }
    }

    pub fn title_right(mut self, right: &'a str) -> Self {
        self.title_right = Some(right);
        self
    }

    pub fn style(mut self, style: PaneStyle) -> Self {
        self.style = style;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.style.is_active = active;
        self
    }

    /// Render the pane header and return the inner content area
    pub fn render_header(&self, f: &mut Frame, area: Rect) -> Rect {
        if let Some(right) = self.title_right {
            theme::render_panel_header_split(f, area, self.title_left, right, self.style.is_active)
        } else {
            theme::render_panel_header(f, area, self.title_left, self.style.is_active)
        }
    }

    /// Render a scrollbar in the given area
    pub fn render_scrollbar(
        f: &mut Frame,
        area: Rect,
        state: &mut ScrollbarState,
        is_active: bool,
    ) {
        if area.width < 2 {
            return;
        }

        f.render_stateful_widget(
            theme::scrollbar(is_active),
            theme::scrollbar_area(area),
            state,
        );
    }

    /// Calculate scrollbar state for a given content/viewport
    pub fn scrollbar_state(
        content_length: usize,
        viewport_length: usize,
        position: usize,
    ) -> ScrollbarState {
        ScrollbarState::default()
            .content_length(content_length)
            .viewport_content_length(viewport_length)
            .position(position)
    }
}
