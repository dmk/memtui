use super::Component;
use crate::action::Action;
use crate::formatter::{Formatter, JsonFormatter, TextFormatter};
use crate::types::Value;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct ValueViewerProps<'a> {
    pub selected_value: Option<&'a Value>,
    pub error_message: Option<&'a String>,
    pub json_formatter: &'a JsonFormatter,
    pub text_formatter: &'a TextFormatter,
    pub is_active: bool,
}

pub struct ValueViewer;

impl ValueViewer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValueViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ValueViewer {
    type Props<'a> = ValueViewerProps<'a>;
    type Msg = Action;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let (lines, style) = if let Some(err) = props.error_message {
            // Error message
            (
                vec![Line::from(format!("Error: {}", err))],
                Style::default().fg(Color::Red),
            )
        } else if let Some(value) = props.selected_value {
            // Try JSON formatting first
            if props.json_formatter.can_format(value) {
                match props.json_formatter.format_to_lines(value) {
                    Ok(json_lines) => (json_lines, Style::default()),
                    Err(_) => {
                        // Fallback to text formatter
                        match props.text_formatter.format(value) {
                            Ok(text) => (
                                text.lines().map(|l| Line::from(l.to_string())).collect(),
                                Style::default(),
                            ),
                            Err(_) => (
                                vec![Line::from("<formatting error>")],
                                Style::default().fg(Color::Red),
                            ),
                        }
                    }
                }
            } else {
                // Use text formatter
                match props.text_formatter.format(value) {
                    Ok(text) => (
                        text.lines().map(|l| Line::from(l.to_string())).collect(),
                        Style::default(),
                    ),
                    Err(_) => (
                        vec![Line::from("<formatting error>")],
                        Style::default().fg(Color::Red),
                    ),
                }
            }
        } else {
            // No value selected
            (
                vec![Line::from("Select a key to view its value")],
                Style::default().fg(Color::DarkGray),
            )
        };

        let value_widget = Paragraph::new(lines)
            .style(style)
            .block(
                Block::default()
                    .title("Value Viewer")
                    .borders(Borders::ALL)
                    .border_style(if props.is_active {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    }),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(value_widget, area);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        None
    }
}
