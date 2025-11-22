use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::Component;
use crate::app::{ConnectionManager, ConnectionStatus};

pub struct StatusBarProps<'a> {
    pub connection_manager: &'a ConnectionManager,
    pub loading_message: Option<&'a str>,
}

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for StatusBar {
    type Props<'a> = StatusBarProps<'a>;
    type Msg = ();

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        // Build connection status (always shown on the left)
        let (conn_style, conn_symbol, conn_text) =
            if let Some(status) = props.connection_manager.get_active_status() {
                match status {
                    ConnectionStatus::Connected => {
                        let text = if let Some(id) = props.connection_manager.get_active_id() {
                            if let Some(config) = props.connection_manager.get_config(id) {
                                format!("{} ({}:{})", config.name, config.host, config.port)
                            } else {
                                "Connected".to_string()
                            }
                        } else {
                            "Connected".to_string()
                        };
                        (Style::default().fg(Color::Green), "● ", text)
                    }
                    ConnectionStatus::Connecting => (
                        Style::default().fg(Color::Yellow),
                        "◐ ",
                        "Connecting...".to_string(),
                    ),
                    ConnectionStatus::Disconnected => (
                        Style::default().fg(Color::DarkGray),
                        "○ ",
                        "Not connected".to_string(),
                    ),
                    ConnectionStatus::Error(ref msg) => (
                        Style::default().fg(Color::Red),
                        "✗ ",
                        format!("Error: {}", msg),
                    ),
                }
            } else {
                (
                    Style::default().fg(Color::DarkGray),
                    "○ ",
                    "No connection selected".to_string(),
                )
            };

        // Build operation indicator (shown on the right if visible)
        let operation_spans = if let Some(msg) = props.loading_message {
            Some(vec![
                Span::raw("  "),
                Span::styled(msg, Style::default().fg(Color::Cyan)),
            ])
        } else {
            None
        };

        // Calculate available width for connection text
        let help_text = " | / search | ? help | q quit";
        let op_width = operation_spans.as_ref().map_or(0, |spans| {
            spans.iter().map(|s| s.content.len()).sum::<usize>()
        });
        let total_reserved = conn_symbol.len() + help_text.len() + op_width;
        let available = area.width as usize - total_reserved;

        // Truncate connection text if needed
        let truncated_conn_text = if conn_text.len() > available {
            format!("{}...", &conn_text[..available.saturating_sub(3)])
        } else {
            conn_text
        };

        // Build final status line
        let mut status_spans = vec![
            Span::styled(conn_symbol, conn_style),
            Span::raw(truncated_conn_text),
            Span::raw(help_text),
        ];

        // Add operation indicator on the right if present
        if let Some(mut op_spans) = operation_spans {
            status_spans.append(&mut op_spans);
        }

        let status_line = Line::from(status_spans);
        let status_bar = Paragraph::new(status_line).style(Style::default().bg(Color::Black));

        f.render_widget(status_bar, area);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        None
    }
}
