use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

use super::components::{
    Component, connection_list::ConnectionListProps, key_browser::KeyBrowserProps,
    value_viewer::ValueViewerProps,
};
use super::render_connection_form;
use super::state::{Panel, UiState};
use crate::app::AppState;

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

    ui_state.last_connection_area = Some(chunks[0]);
    ui_state.last_key_area = Some(chunks[1]);
    ui_state.last_value_area = Some(chunks[2]);

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
        super::render::render_help(f);
    }
}

fn render_connections(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    let props = ConnectionListProps {
        configs: app_state.connection_manager.get_configs(),
        active_id: app_state.connection_manager.get_active_id(),
        statuses: app_state.connection_manager.get_statuses(),
        is_active: ui_state.active_panel == Panel::Connections,
    };

    ui_state.connection_list.render(f, area, props);
}

fn render_keys(f: &mut Frame, ui_state: &mut UiState, app_state: &mut AppState, area: Rect) {
    // Update viewport height in app_state for logic that depends on it (like scrolling logic in AppState)
    // Note: KeyBrowser also calculates it internally for rendering.
    app_state.viewport_height = area.height.saturating_sub(2) as usize;

    let props = KeyBrowserProps {
        keys: &app_state.keys,
        total_count: app_state.total_key_count,
        is_loading: app_state.is_loading_keys,
        active_search_query: None, // TODO: Implement search query
        is_active: ui_state.active_panel == Panel::Keys,
    };

    ui_state.key_browser.render(f, area, props);
}

fn render_value(f: &mut Frame, ui_state: &mut UiState, app_state: &AppState, area: Rect) {
    let selected_key_type = app_state
        .selected_key_index
        .and_then(|idx| app_state.keys.get(idx))
        .and_then(|k| k.as_ref())
        .map(|k| k.value_type);

    let props = ValueViewerProps {
        selected_value: app_state.selected_value.as_ref(),
        selected_key_type,
        error_message: app_state.error_message.as_ref(),
        json_formatter: &app_state.json_formatter,
        text_formatter: &app_state.text_formatter,
        is_active: ui_state.active_panel == Panel::Value,
    };

    ui_state.value_viewer.render(f, area, props);
}

pub fn render_help(f: &mut Frame) {
    use ratatui::{
        layout::Alignment,
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };

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
    use crate::app::ConnectionStatus;
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };

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
