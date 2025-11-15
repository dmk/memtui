use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::render_connection_form;
use super::state::{Panel, UiState};
use crate::app::AppState;
use crate::types::KeyMetadata;

/// Main UI rendering function
pub fn render(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState) {
    // Create three-panel layout
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(f.area());

    // Left panel: Connections
    render_connections(f, app_state, ui_state, chunks[0]);

    // Middle panel: Key Browser
    render_keys(f, ui_state, &app_state.keys, chunks[1]);

    // Right panel: Value Viewer
    render_value(
        f,
        ui_state,
        &app_state.selected_value,
        app_state.error_message.as_deref(),
        chunks[2],
    );

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
            let text = format!("{} ({}:{})", config.name, config.host, config.port);
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

fn render_value(
    f: &mut Frame,
    ui_state: &UiState,
    selected_value: &str,
    error: Option<&str>,
    area: Rect,
) {
    let value_text = if let Some(err) = error {
        format!("Error: {}", err)
    } else if !selected_value.is_empty() {
        selected_value.to_string()
    } else {
        "Select a key to view its value".to_string()
    };

    let style = if error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };

    let value = Paragraph::new(value_text)
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

    f.render_widget(value, area);
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
