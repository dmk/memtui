use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::render_connection_form;
use super::state::{Panel, UiState};
use crate::app::{AppState, ConnectionStatus};
use crate::formatter::Formatter;
use crate::types::KeyMetadata;

/// Main UI rendering function
pub fn render(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState) {
    // Create main layout with status bar at the bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    // Create three-panel layout
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(main_chunks[0]);

    // Left panel: Connections
    render_connections(f, app_state, ui_state, chunks[0]);

    // Middle panel: Key Browser
    render_keys(f, ui_state, &app_state.keys, chunks[1]);

    // Right panel: Value Viewer
    render_value(f, ui_state, app_state, chunks[2]);

    // Bottom: Status bar
    render_status_bar(f, app_state, main_chunks[1]);

    // Show modals
    if ui_state.show_connection_form {
        render_connection_form(f, &ui_state.connection_form, ui_state.form_error.as_deref());
    } else if ui_state.show_help {
        render_help(f);
    }
}

fn render_connections(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    let configs = app_state.connection_manager.get_configs();

    let connections: Vec<ListItem> = configs
        .iter()
        .map(|config| {
            let status = app_state.connection_manager.get_status(&config.id);
            let status_indicator = match status {
                ConnectionStatus::Connected => "●",
                ConnectionStatus::Connecting => "◐",
                ConnectionStatus::Disconnected => "○",
                ConnectionStatus::Error(_) => "✗",
            };
            let text = format!(
                "{} {} ({}:{})",
                status_indicator, config.name, config.host, config.port
            );
            ListItem::new(text)
        })
        .collect();

    let connections_list = List::new(connections)
        .block(
            Block::default()
                .title("Connections")
                .borders(Borders::ALL)
                .border_style(if ui_state.active_panel == Panel::Connections {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(connections_list, area, &mut ui_state.connection_state);
}

fn render_keys(f: &mut Frame, ui_state: &mut UiState, keys: &[KeyMetadata], area: Rect) {
    let key_items: Vec<ListItem> = keys
        .iter()
        .map(|k| ListItem::new(k.name.as_str()))
        .collect();

    let key_count = keys.len();
    let keys_list = List::new(key_items)
        .block(
            Block::default()
                .title(format!("Key Browser [{} keys]", key_count))
                .borders(Borders::ALL)
                .border_style(if ui_state.active_panel == Panel::Keys {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(keys_list, area, &mut ui_state.key_state);
}

fn render_value(f: &mut Frame, ui_state: &UiState, app_state: &AppState, area: Rect) {
    let (lines, style) = if let Some(err) = &app_state.error_message {
        // Error message
        (
            vec![Line::from(format!("Error: {}", err))],
            Style::default().fg(Color::Red),
        )
    } else if let Some(value) = &app_state.selected_value {
        // Try JSON formatting first
        if app_state.json_formatter.can_format(value) {
            match app_state.json_formatter.format_to_lines(value) {
                Ok(json_lines) => (json_lines, Style::default()),
                Err(_) => {
                    // Fallback to text formatter
                    match app_state.text_formatter.format(value) {
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
            match app_state.text_formatter.format(value) {
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
                .border_style(if ui_state.active_panel == Panel::Value {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(value_widget, area);
}

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 50, f.area());

    let help_text = vec![
        Line::from(Span::styled(
            "memtui - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("Tab         ", Style::default().fg(Color::Yellow)),
            Span::raw("Next panel"),
        ]),
        Line::from(vec![
            Span::styled("Shift+Tab   ", Style::default().fg(Color::Yellow)),
            Span::raw("Previous panel"),
        ]),
        Line::from(vec![
            Span::styled("↑/k         ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("↓/j         ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Connections",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("n           ", Style::default().fg(Color::Yellow)),
            Span::raw("New connection"),
        ]),
        Line::from(vec![
            Span::styled("Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw("Connect/Disconnect"),
        ]),
        Line::from(vec![
            Span::styled("d           ", Style::default().fg(Color::Yellow)),
            Span::raw("Delete connection"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "General",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("?           ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle help"),
        ]),
        Line::from(vec![
            Span::styled("q/Esc       ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    f.render_widget(Clear, area);
    f.render_widget(help, area);
}

fn render_status_bar(f: &mut Frame, app_state: &AppState, area: Rect) {
    let status_text = if let Some(status) = app_state.connection_manager.get_active_status() {
        match status {
            ConnectionStatus::Connected => {
                if let Some(id) = app_state.connection_manager.get_active_id() {
                    if let Some(config) = app_state.connection_manager.get_config(id) {
                        format!(
                            "Connected to {} ({}:{})",
                            config.name, config.host, config.port
                        )
                    } else {
                        "Connected".to_string()
                    }
                } else {
                    "Connected".to_string()
                }
            }
            ConnectionStatus::Connecting => "Connecting...".to_string(),
            ConnectionStatus::Disconnected => "Not connected".to_string(),
            ConnectionStatus::Error(ref msg) => format!("Error: {}", msg),
        }
    } else {
        "No connection selected".to_string()
    };

    let (style, status_label) =
        if let Some(status) = app_state.connection_manager.get_active_status() {
            match status {
                ConnectionStatus::Connected => (Style::default().fg(Color::Green), "● "),
                ConnectionStatus::Connecting => (Style::default().fg(Color::Yellow), "◐ "),
                ConnectionStatus::Disconnected => (Style::default().fg(Color::DarkGray), "○ "),
                ConnectionStatus::Error(_) => (Style::default().fg(Color::Red), "✗ "),
            }
        } else {
            (Style::default().fg(Color::DarkGray), "○ ")
        };

    let status_line = Line::from(vec![
        Span::styled(status_label, style),
        Span::raw(status_text),
        Span::raw(" | Press ? for help | q to quit"),
    ]);

    let status_bar = Paragraph::new(status_line).style(Style::default().bg(Color::Black));

    f.render_widget(status_bar, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
