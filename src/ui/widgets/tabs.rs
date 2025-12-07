//! Tabs widget - horizontal tab bar
//!
//! A tab bar for switching between multiple views.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme;

/// A single tab item
#[derive(Debug, Clone)]
pub struct TabItem {
    /// Unique identifier for the tab
    pub id: String,
    /// Display label
    pub label: String,
    /// Whether the tab is active
    pub is_active: bool,
}

impl TabItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            is_active: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }
}

/// A horizontal tab bar widget
pub struct TabBar<'a> {
    tabs: &'a [TabItem],
}

impl<'a> TabBar<'a> {
    pub fn new(tabs: &'a [TabItem]) -> Self {
        Self { tabs }
    }

    /// Render the tab bar
    pub fn render(self, f: &mut Frame, area: Rect) -> Vec<TabRegion> {
        let mut regions = Vec::new();
        let mut spans = Vec::new();
        let mut cursor_x = area.x + 1;
        let max_x = area.x.saturating_add(area.width);

        if self.tabs.is_empty() {
            let tabs_line = Paragraph::new(Line::from(vec![Span::styled(
                " Ctrl+P to open connections",
                Style::default().fg(theme::TEXT_DIM()),
            )]))
            .style(theme::util_bg());
            f.render_widget(tabs_line, area);
            return regions;
        }

        spans.push(Span::raw(" "));

        for tab in self.tabs {
            let label = format!(" {} ", tab.label);
            let width = label.chars().count() as u16;

            if cursor_x.saturating_add(width + 2) > max_x {
                break;
            }

            let tab_area = Rect::new(cursor_x, area.y, width.max(1), 1);
            regions.push(TabRegion {
                id: tab.id.clone(),
                area: tab_area,
            });

            if tab.is_active {
                spans.push(Span::styled(label, theme::tab_active()));
            } else {
                spans.push(Span::styled(
                    label,
                    Style::default().fg(theme::TEXT_SECONDARY()),
                ));
            }
            spans.push(Span::raw(" "));
            cursor_x = cursor_x.saturating_add(width + 1);
        }

        let tabs_line = Paragraph::new(Line::from(spans)).style(theme::util_bg());
        f.render_widget(tabs_line, area);

        regions
    }
}

/// Region info for a rendered tab (for click handling)
#[derive(Debug, Clone)]
pub struct TabRegion {
    pub id: String,
    pub area: Rect,
}

impl TabRegion {
    /// Check if a point is within this tab region
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.area.x
            && x < self.area.x.saturating_add(self.area.width)
            && y >= self.area.y
            && y < self.area.y.saturating_add(self.area.height)
    }
}
