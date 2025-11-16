use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
    },
};

use super::render_connection_form;
use super::state::{Panel, UiState};
use crate::app::{AppState, ConnectionStatus};
use crate::formatter::Formatter;

/// Main UI rendering function
pub fn render(f: &mut Frame, app_state: &mut AppState, ui_state: &mut UiState) {
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
    render_keys(f, ui_state, app_state, chunks[1]);

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

fn render_keys(f: &mut Frame, ui_state: &mut UiState, app_state: &mut AppState, area: Rect) {
    // Calculate viewport height (subtract borders and title)
    let viewport_height = area.height.saturating_sub(2) as usize;
    app_state.viewport_height = viewport_height;

    // Virtualized: build only the visible window around the selected index
    let total_count = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or_else(|| app_state.keys.len());

    let selected_abs = ui_state
        .key_state
        .selected()
        .unwrap_or(0)
        .min(total_count.saturating_sub(1));
    let visible_len = viewport_height.min(total_count.max(1));

    // Calculate viewport start to keep selection visible and scroll smoothly
    // Strategy: keep selection at a fixed position in the viewport (1/2 from top)
    let scroll_offset = visible_len / 2;
    let max_start = total_count.saturating_sub(visible_len);

    let start_index = if total_count == 0 || visible_len == 0 {
        0
    } else {
        // Calculate start_index so that selected_abs appears at scroll_offset position
        let ideal_start = selected_abs.saturating_sub(scroll_offset);
        // Clamp to valid range [0, max_start]
        ideal_start.min(max_start).max(0)
    };

    let mut key_items: Vec<ListItem> = Vec::with_capacity(visible_len);
    for offset in 0..visible_len {
        let abs = start_index + offset;
        if let Some(Some(k)) = app_state.keys.get(abs) {
            key_items.push(ListItem::new(k.name.as_str()));
        } else {
            key_items.push(ListItem::new(Span::styled(
                "...",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Build title with static total count + current position
    let (title, position_suffix) = if let Some(total) = app_state.total_key_count {
        (format!("Key Browser [{} keys]", total), total as usize)
    } else {
        (format!("Key Browser [{} keys]", total_count), total_count)
    };

    let position = ui_state
        .key_state
        .selected()
        .map(|i| format!(" [{} / {}]", i.saturating_add(1), position_suffix))
        .unwrap_or_else(|| " [—]".to_string());

    // Add loading indicator if loading
    let title_with_status = if app_state.is_loading_keys {
        format!("{}{} [Loading...]", title, position)
    } else {
        format!("{}{}", title, position)
    };

    let keys_list = List::new(key_items)
        .block(
            Block::default()
                .title(title_with_status)
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

    // Use a local state with relative selection for the visible window
    let mut view_state = ui_state.key_state.clone();
    if total_count > 0 {
        let rel = selected_abs.saturating_sub(start_index);
        view_state.select(Some(rel));
    } else {
        view_state.select(None);
    }
    f.render_stateful_widget(keys_list, area, &mut view_state);

    // Render scrollbar - represents full list
    if total_count > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let current_position = selected_abs;

        ui_state.key_scrollbar_state = ui_state
            .key_scrollbar_state
            .content_length(total_count)
            .position(current_position);

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut ui_state.key_scrollbar_state);
    }
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
