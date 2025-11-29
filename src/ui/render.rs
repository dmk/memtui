use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use super::components::{
    connection_list::ConnectionListProps,
    key_browser::KeyBrowserProps,
    value_viewer::ValueViewerProps,
    warning_message::{MessageKind, WarningMessage, WarningMessageProps},
    welcome::WelcomeScreenProps,
    Component,
};
use super::render_connection_form;
use super::state::{Panel, TabRegion, UiState};
use crate::app::{AppState, ConnectionStatus};
use crate::types::BackendType;

/// Main UI rendering function
pub fn render(f: &mut Frame, app_state: &mut AppState, ui_state: &mut UiState) {
    let configs = app_state.connection_manager.get_configs();
    let open_configs: Vec<_> = configs
        .iter()
        .filter(|c| {
            let status = app_state.connection_manager.get_status(&c.id);
            !matches!(status, ConnectionStatus::Disconnected)
        })
        .collect();

    let show_tabs = open_configs.len() > 1;

    // Check if we should show warning for memcached
    let show_memcached_warning = app_state
        .connection_manager
        .get_active_id()
        .and_then(|id| app_state.connection_manager.get_config(id))
        .map(|config| config.backend_type == BackendType::Memcached)
        .unwrap_or(false);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if show_tabs { 2 } else { 0 }),
            Constraint::Length(if show_memcached_warning { 1 } else { 0 }),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    let tab_area = root[0];
    let warning_area = root[1];
    let body_area = root[2];
    let status_area = root[3];

    if show_tabs {
        render_tabs(f, app_state, ui_state, tab_area);
    }

    if show_memcached_warning {
        let mut warning_component = WarningMessage::new();
        let warning_props = WarningMessageProps {
            kind: MessageKind::Warning,
            message: "Note: Memcached doesn't provide native key listing. The keys list may not be consistent.",
        };
        warning_component.render(f, warning_area, warning_props);
    }

    if app_state.connection_manager.get_active_id().is_some() {
        render_body(f, app_state, ui_state, body_area);
    } else {
        ui_state.last_key_area = None;
        ui_state.last_value_area = None;
        render_welcome(f, app_state, ui_state, body_area);
    }

    render_status_bar(f, app_state, status_area);

    if ui_state.show_connection_palette {
        render_connection_palette(f, app_state, ui_state);
    }

    if ui_state.show_connection_form {
        render_connection_form(f, &ui_state.connection_form, ui_state.form_error.as_deref());
    } else if ui_state.show_help {
        super::render::render_help(f);
    }

    // Quit confirmation should be rendered on top of everything
    if ui_state.show_quit_confirmation {
        render_quit_confirmation(f);
    }
}

fn render_tabs(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    ui_state.tab_regions.clear();
    ui_state.tab_bar_area = Some(area);

    let mut spans = Vec::new();
    let mut cursor_x = area.x;
    let max_x = area.x.saturating_add(area.width);

    let configs = app_state.connection_manager.get_configs();
    let open_configs: Vec<_> = configs
        .iter()
        .filter(|c| {
            let status = app_state.connection_manager.get_status(&c.id);
            !matches!(status, ConnectionStatus::Disconnected)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(block, area);

    if open_configs.is_empty() {
        // If we are somehow here (maybe active is None but show_tabs was somehow called?), return
        return;
    }

    for config in open_configs {
        let status = app_state.connection_manager.get_status(&config.id);
        let (icon, tone) = match status {
            ConnectionStatus::Connected => ("●", Color::Green),
            ConnectionStatus::Connecting => ("◐", Color::Yellow),
            ConnectionStatus::Disconnected => ("○", Color::DarkGray),
            ConnectionStatus::Error(_) => ("✗", Color::Red),
        };

        let is_active = app_state
            .connection_manager
            .get_active_id()
            .map(|id| id == config.id.as_str())
            .unwrap_or(false);

        let label = format!(" {} {} ", icon, config.name);
        let width = label.chars().count() as u16;
        if cursor_x.saturating_add(width) > max_x {
            break;
        }

        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone)
        };

        let tab_area = Rect::new(cursor_x, area.y, width.max(1), area.height);
        ui_state.tab_regions.push(TabRegion {
            id: config.id.clone(),
            area: tab_area,
        });
        spans.push(Span::styled(label, style));
        spans.push(Span::raw("  "));
        cursor_x = cursor_x.saturating_add(width + 2);
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            "Ctrl+P to open connections",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let tabs_line = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);
    f.render_widget(tabs_line, area);
}

fn render_body(f: &mut Frame, app_state: &mut AppState, ui_state: &mut UiState, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    ui_state.last_key_area = Some(body_chunks[0]);
    ui_state.last_value_area = Some(body_chunks[1]);

    render_keys(f, ui_state, app_state, body_chunks[0]);
    render_value(f, ui_state, app_state, body_chunks[1]);
}

fn render_keys(f: &mut Frame, ui_state: &mut UiState, app_state: &mut AppState, area: Rect) {
    app_state.viewport_height = area.height.saturating_sub(2) as usize;

    let backend_type = app_state
        .connection_manager
        .get_active_id()
        .and_then(|id| app_state.connection_manager.get_config(id))
        .map(|config| config.backend_type);

    // Search state
    let active_search_query = if app_state.is_searching || !app_state.search_query.is_empty() {
        Some(app_state.search_query.as_str())
    } else {
        None
    };

    let props = KeyBrowserProps {
        keys: &app_state.keys,
        total_count: app_state.total_key_count,
        is_loading: app_state.is_loading_keys,
        active_search_query,
        is_active: ui_state.active_panel == Panel::Keys,
        backend_type,
        is_searching: app_state.is_searching,
        search_results_local: &app_state.search_results_local,
        search_results_server: &app_state.search_results_server,
        is_server_searching: app_state.is_server_searching,
        search_selection_index: app_state.search_selection_index,
    };

    ui_state.key_browser.render(f, area, props);
}

fn render_value(f: &mut Frame, ui_state: &mut UiState, app_state: &AppState, area: Rect) {
    let selected_key_type = app_state
        .selected_key_index
        .and_then(|idx| app_state.keys.get(idx))
        .and_then(|k| k.as_ref())
        .map(|k| k.value_type);

    let backend_type = app_state
        .connection_manager
        .get_active_id()
        .and_then(|id| app_state.connection_manager.get_config(id))
        .map(|config| config.backend_type);

    let props = ValueViewerProps {
        selected_value: app_state.selected_value.as_ref(),
        selected_key_type,
        error_message: app_state.error_message.as_ref(),
        json_formatter: &app_state.json_formatter,
        text_formatter: &app_state.text_formatter,
        is_active: ui_state.active_panel == Panel::Value,
        backend_type,
    };

    ui_state.value_viewer.render(f, area, props);
}

fn render_welcome(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    let configs = app_state.connection_manager.get_configs();
    let recent_configs = ui_state
        .recent_connection_ids
        .iter()
        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
        .collect();

    let props = WelcomeScreenProps { recent_configs };
    ui_state.welcome_screen.render(f, area, props);
}

fn render_connection_palette(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState) {
    let area = centered_rect(60, 70, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .margin(1)
        .split(area);

    ui_state.connection_palette_area = Some(chunks[0]);

    let props = ConnectionListProps {
        configs: app_state.connection_manager.get_configs(),
        active_id: app_state.connection_manager.get_active_id(),
        statuses: app_state.connection_manager.get_statuses(),
        is_active: true,
    };

    f.render_widget(Clear, area);
    ui_state.connection_list.render(f, chunks[0], props);

    let instructions = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" connect / focus   "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" delete   "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" close"),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(instructions, chunks[1]);
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
            Span::styled("Ctrl+P      ", Style::default().fg(Color::Yellow)),
            Span::raw("Open palette"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+N      ", Style::default().fg(Color::Yellow)),
            Span::raw("New connection"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+Tab    ", Style::default().fg(Color::Yellow)),
            Span::raw("Next connection tab"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+BackTab", Style::default().fg(Color::Yellow)),
            Span::raw("Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw("Connect / focus"),
        ]),
        Line::from(vec![
            Span::styled("d           ", Style::default().fg(Color::Yellow)),
            Span::raw("Delete selected"),
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
                .border_type(BorderType::Rounded)
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
        Span::raw("  |  Ctrl+Tab next tab  |  Ctrl+P palette  |  Ctrl+N new  |  ? help  |  q quit"),
    ]);

    let status_bar = Paragraph::new(status_line).style(Style::default().bg(Color::Black));
    f.render_widget(status_bar, area);
}

fn render_quit_confirmation(f: &mut Frame) {
    // Small centered dialog
    let area = centered_rect(40, 20, f.area());
    // Clamp to reasonable size
    let area = Rect {
        x: area.x,
        y: area.y,
        width: area.width.min(50).max(30),
        height: area.height.min(7).max(5),
    };
    // Re-center with clamped size
    let area = Rect {
        x: (f.area().width.saturating_sub(area.width)) / 2,
        y: (f.area().height.saturating_sub(area.height)) / 2,
        width: area.width,
        height: area.height,
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to quit?",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" yes  "),
            Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" no  "),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Quit ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(dialog, area);
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
