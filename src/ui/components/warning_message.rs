use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::Component;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Warning,
    Error,
}

pub struct WarningMessageProps<'a> {
    pub kind: MessageKind,
    pub message: &'a str,
}

pub struct WarningMessage;

impl WarningMessage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WarningMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for WarningMessage {
    type Props<'a> = WarningMessageProps<'a>;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let (text_style, bg_style) = match props.kind {
            MessageKind::Warning => theme::message_warning(),
            MessageKind::Error => theme::message_error(),
        };

        let icon = match props.kind {
            MessageKind::Warning => "⚠ ",
            MessageKind::Error => "✕ ",
        };

        let message_line = Line::from(vec![
            Span::styled(icon, text_style),
            Span::styled(props.message, text_style),
        ]);

        let paragraph = Paragraph::new(message_line)
            .style(bg_style)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}
