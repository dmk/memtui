use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::Component;

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
    type Msg = ();

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let (icon, style) = match props.kind {
            MessageKind::Warning => ("⚠ ", Style::default().fg(Color::Yellow)),
            MessageKind::Error => ("✗ ", Style::default().fg(Color::Red)),
        };

        let message_line = Line::from(vec![
            Span::styled(icon, style),
            Span::styled(props.message, style),
        ]);

        let bg_style = match props.kind {
            MessageKind::Warning => Style::default().bg(Color::Rgb(40, 30, 0)),
            MessageKind::Error => Style::default().bg(Color::Rgb(40, 0, 0)),
        };

        let paragraph = Paragraph::new(message_line)
            .style(bg_style)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        None
    }
}
