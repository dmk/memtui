//! Modal widget - centered overlay with backdrop
//!
//! Modals use raw (undimmed) theme colors while the background is dimmed.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

use crate::ui::theme::raw;

/// Style configuration for a Modal
#[derive(Debug, Clone, Default)]
pub struct ModalStyle {
    /// Width as percentage of screen (0-100)
    pub width_percent: u16,
    /// Height as percentage of screen (0-100)
    pub height_percent: u16,
    /// Minimum width in characters
    pub min_width: u16,
    /// Maximum width in characters
    pub max_width: u16,
    /// Minimum height in rows
    pub min_height: u16,
    /// Maximum height in rows
    pub max_height: u16,
}

impl ModalStyle {
    pub fn new() -> Self {
        Self {
            width_percent: 60,
            height_percent: 70,
            min_width: 30,
            max_width: 100,
            min_height: 10,
            max_height: 40,
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width_percent = width;
        self.height_percent = height;
        self
    }

    pub fn min_size(mut self, width: u16, height: u16) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    pub fn max_size(mut self, width: u16, height: u16) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }
}

/// A centered modal overlay
pub struct Modal<'a> {
    title: &'a str,
    style: ModalStyle,
}

impl<'a> Modal<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            style: ModalStyle::new(),
        }
    }

    pub fn style(mut self, style: ModalStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.style = self.style.size(width, height);
        self
    }

    /// Calculate the modal area centered in the given screen area
    pub fn calculate_area(&self, screen: Rect) -> Rect {
        let area = centered_rect(self.style.width_percent, self.style.height_percent, screen);

        // Apply min/max constraints
        let width = area.width.clamp(self.style.min_width, self.style.max_width);
        let height = area
            .height
            .clamp(self.style.min_height, self.style.max_height);

        // Re-center with constrained size
        Rect {
            x: screen.x + (screen.width.saturating_sub(width)) / 2,
            y: screen.y + (screen.height.saturating_sub(height)) / 2,
            width,
            height,
        }
    }

    /// Render the modal and return the inner area for content
    pub fn render(&self, f: &mut Frame, screen: Rect) -> Rect {
        let area = self.calculate_area(screen);

        // Clear the modal area
        f.render_widget(Clear, area);

        // Render the modal block with raw (undimmed) colors
        let block = raw::modal_block(self.title);
        let inner = block.inner(area);
        f.render_widget(block, area);

        inner
    }
}

/// Calculate a centered rectangle within the given area
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
