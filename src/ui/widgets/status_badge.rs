//! StatusBadge widget - connection/status indicator
//!
//! A compact badge showing status with an icon and optional text.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{self, AnimationState};

/// Kind of status to display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

/// A status indicator badge
pub struct StatusBadge<'a> {
    kind: StatusKind,
    text: Option<&'a str>,
    animation: Option<&'a AnimationState>,
}

impl<'a> StatusBadge<'a> {
    pub fn new(kind: StatusKind) -> Self {
        Self {
            kind,
            text: None,
            animation: None,
        }
    }

    pub fn connected() -> Self {
        Self::new(StatusKind::Connected)
    }

    pub fn connecting() -> Self {
        Self::new(StatusKind::Connecting)
    }

    pub fn disconnected() -> Self {
        Self::new(StatusKind::Disconnected)
    }

    pub fn error() -> Self {
        Self::new(StatusKind::Error)
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    pub fn animation(mut self, animation: &'a AnimationState) -> Self {
        self.animation = Some(animation);
        self
    }

    /// Get the icon for the current status
    fn icon(&self) -> &'static str {
        match self.kind {
            StatusKind::Connected => theme::INDICATOR_CONNECTED,
            StatusKind::Connecting => {
                if let Some(anim) = self.animation {
                    theme::spinner_pulse(anim)
                } else {
                    theme::INDICATOR_CONNECTED
                }
            }
            StatusKind::Disconnected => theme::INDICATOR_DISCONNECTED,
            StatusKind::Error => theme::INDICATOR_ERROR,
        }
    }

    /// Get the style for the current status
    fn style(&self) -> Style {
        match self.kind {
            StatusKind::Connected => theme::status_connected(),
            StatusKind::Connecting => {
                if let Some(anim) = self.animation {
                    theme::status_connecting(anim)
                } else {
                    theme::status_connecting(&AnimationState::new())
                }
            }
            StatusKind::Disconnected => theme::status_disconnected(),
            StatusKind::Error => theme::status_error(),
        }
    }

    /// Render the status badge
    pub fn render(self, f: &mut Frame, area: Rect) {
        let icon = self.icon();
        let style = self.style();

        let spans = if let Some(text) = self.text {
            vec![
                Span::styled(icon, style),
                Span::styled(format!(" {}", text), Style::default().fg(theme::TEXT_DIM())),
            ]
        } else {
            vec![Span::styled(icon, style)]
        };

        let widget = Paragraph::new(Line::from(spans));
        f.render_widget(widget, area);
    }
}
