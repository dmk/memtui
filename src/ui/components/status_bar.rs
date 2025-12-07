use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::Component;
use crate::app::{ConnectionManager, ConnectionStatus};
use crate::ui::theme;

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

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        // Build connection status (always shown on the left)
        let (conn_symbol, conn_style, conn_text) =
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
                        let (symbol, style) = theme::status_indicator_connected();
                        (symbol, style, text)
                    }
                    ConnectionStatus::Connecting => {
                        let (symbol, style) = theme::status_indicator_connecting();
                        (symbol, style, "Connecting...".to_string())
                    }
                    ConnectionStatus::Disconnected => {
                        let (symbol, style) = theme::status_indicator_disconnected();
                        (symbol, style, "Not connected".to_string())
                    }
                    ConnectionStatus::Error(ref msg) => {
                        let (symbol, style) = theme::status_indicator_error();
                        (symbol, style, format!("Error: {}", msg))
                    }
                }
            } else {
                let (symbol, style) = theme::status_indicator_disconnected();
                (symbol, style, "No connection selected".to_string())
            };

        // Build operation indicator (shown on the right if visible)
        let operation_spans = if let Some(msg) = props.loading_message {
            Some(vec![
                Span::raw("  "),
                Span::styled(msg, theme::status_operation()),
            ])
        } else {
            None
        };

        // Calculate available width for connection text
        let help_text = " | / search | ? help | q quit";
        let op_width = operation_spans.as_ref().map_or(0, |spans| {
            spans.iter().map(|s| s.content.len()).sum::<usize>()
        });
        let total_reserved = conn_symbol.len() + 1 + help_text.len() + op_width;
        let available = (area.width as usize).saturating_sub(total_reserved);

        // Truncate connection text if needed
        let truncated_conn_text = if conn_text.len() > available {
            format!("{}...", &conn_text[..available.saturating_sub(3)])
        } else {
            conn_text
        };

        // Build final status line
        let mut status_spans = vec![
            Span::styled(format!("{} ", conn_symbol), conn_style),
            Span::styled(truncated_conn_text, theme::status_hint()),
            Span::styled(help_text, theme::status_hint()),
        ];

        // Add operation indicator on the right if present
        if let Some(mut op_spans) = operation_spans {
            status_spans.append(&mut op_spans);
        }

        let status_line = Line::from(status_spans);
        let status_bar = Paragraph::new(status_line).style(theme::util_bg());

        f.render_widget(status_bar, area);
    }
}
