use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use super::components::{
    connection_list::ConnectionListProps,
    help::render_help,
    key_browser::KeyBrowserProps,
    value_viewer::ValueViewerProps,
    warning_message::{MessageKind, WarningMessage, WarningMessageProps},
    welcome::WelcomeScreenProps,
    Component,
};
use super::render_connection_form;
use super::state::{Panel, TabRegion, UiState};
use super::theme::{self, AnimationState};
use crate::app::{AppState, ConnectionStatus};
use crate::keybindings::{BindingContext, KeybindingsConfig, format_key_for_display};
use crate::types::BackendType;

/// Main UI rendering function
pub fn render(f: &mut Frame, app_state: &mut AppState, ui_state: &mut UiState, keybindings: &KeybindingsConfig) {
    // Render the deep background
    let bg_block = Block::default().style(Style::default().bg(theme::BG_DEEP()));
    f.render_widget(bg_block, f.area());

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
            Constraint::Length(if show_tabs { 3 } else { 0 }), // Tabs with more space
            Constraint::Length(if show_memcached_warning { 1 } else { 0 }),
            Constraint::Min(0),    // Body
            Constraint::Length(2), // Status bar with more space
        ])
        .split(f.area());

    let tab_area = root[0];
    let warning_area = root[1];
    let body_area = root[2];
    let status_area = root[3];

    // Store body area for resize calculations
    ui_state.last_body_area = Some(body_area);

    if show_tabs {
        render_tabs(f, app_state, ui_state, tab_area);
    }

    if show_memcached_warning {
        let mut warning_component = WarningMessage::new();
        let warning_props = WarningMessageProps {
            kind: MessageKind::Warning,
            message: "Memcached doesn't provide native key listing. Keys may not be consistent.",
        };
        warning_component.render(f, warning_area, warning_props);
    }

    if app_state.connection_manager.get_active_id().is_some() {
        render_body(f, app_state, ui_state, body_area);
    } else {
        ui_state.last_key_area = None;
        ui_state.last_value_area = None;
        render_welcome(f, app_state, ui_state, body_area, keybindings);
    }

    render_status_bar(f, app_state, ui_state, keybindings, status_area);

    if ui_state.show_connection_palette {
        render_connection_palette(f, app_state, ui_state);
    }

    if ui_state.show_connection_form {
        render_connection_form(f, &ui_state.connection_form, ui_state.form_error.as_deref());
    } else if ui_state.show_help {
        render_help(f, keybindings);
    }

    // Quit confirmation should be rendered on top of everything
    if ui_state.show_quit_confirmation {
        render_quit_confirmation(f, &ui_state.animation);
    }
}

fn render_tabs(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    ui_state.tab_regions.clear();
    ui_state.tab_bar_area = Some(area);

    // Background for tab bar
    let tab_bg = Block::default()
        .style(Style::default().bg(theme::BG_PANEL()))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(40, 50, 70)));
    f.render_widget(tab_bg, area);

    let mut spans = Vec::new();
    let mut cursor_x = area.x + 1;
    let max_x = area.x.saturating_add(area.width);

    let configs = app_state.connection_manager.get_configs();
    let open_configs: Vec<_> = configs
        .iter()
        .filter(|c| {
            let status = app_state.connection_manager.get_status(&c.id);
            !matches!(status, ConnectionStatus::Disconnected)
        })
        .collect();

    if open_configs.is_empty() {
        return;
    }

    for config in open_configs {
        let status = app_state.connection_manager.get_status(&config.id);
        let (icon, tone) = match status {
            ConnectionStatus::Connected => (theme::INDICATOR_CONNECTED, theme::NEON_GREEN()),
            ConnectionStatus::Connecting => {
                let spinner = theme::spinner_pulse(&ui_state.animation);
                (spinner, theme::NEON_AMBER())
            }
            ConnectionStatus::Disconnected => (theme::INDICATOR_DISCONNECTED, theme::TEXT_DIM()),
            ConnectionStatus::Error(_) => (theme::INDICATOR_ERROR, theme::NEON_RED()),
        };

        let is_active = app_state
            .connection_manager
            .get_active_id()
            .map(|id| id == config.id.as_str())
            .unwrap_or(false);

        let label = format!(" {} {} ", icon, config.name);
        let width = label.chars().count() as u16;
        if cursor_x.saturating_add(width + 2) > max_x {
            break;
        }

        let (style, bg_style) = if is_active {
            (
                Style::default()
                    .fg(theme::BG_DEEP())
                    .add_modifier(Modifier::BOLD),
                Some(theme::ACCENT()),
            )
        } else {
            (Style::default().fg(tone), None)
        };

        let tab_area = Rect::new(cursor_x, area.y + 1, width.max(1), 1);
        ui_state.tab_regions.push(TabRegion {
            id: config.id.clone(),
            area: tab_area,
        });

        if let Some(bg) = bg_style {
            spans.push(Span::styled(label, style.bg(bg)));
        } else {
            // Create a subtle background for inactive tabs
            spans.push(Span::styled(label, style.bg(theme::BG_SURFACE())));
        }
        spans.push(Span::styled(" ", Style::default()));
        cursor_x = cursor_x.saturating_add(width + 1);
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            "  Ctrl+P to open connections",
            Style::default().fg(theme::TEXT_DIM()),
        ));
    }

    let tabs_line = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .style(Style::default().bg(theme::BG_PANEL()));

    let inner_area = Rect::new(area.x, area.y + 1, area.width, 1);
    f.render_widget(tabs_line, inner_area);
}

fn render_body(f: &mut Frame, app_state: &mut AppState, ui_state: &mut UiState, area: Rect) {
    // Use the resizable pane split ratio
    let left_percent = ui_state.pane_split.left_percent();
    let right_percent = ui_state.pane_split.right_percent();

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Length(1), // Resize handle
            Constraint::Percentage(right_percent),
        ])
        .split(area);

    ui_state.last_key_area = Some(body_chunks[0]);
    ui_state.last_value_area = Some(body_chunks[2]);

    // Render resize handle
    render_resize_handle(f, body_chunks[1], ui_state.is_resizing, &ui_state.animation);

    render_keys(f, ui_state, app_state, body_chunks[0]);
    render_value(f, ui_state, app_state, body_chunks[2]);
}

fn render_resize_handle(f: &mut Frame, area: Rect, is_active: bool, _animation: &AnimationState) {
    let style = if is_active {
        Style::default().fg(theme::NEON_CYAN())
    } else {
        Style::default().fg(Color::Rgb(50, 60, 80))
    };

    // Draw vertical separator with handle indicator
    let mut lines = Vec::new();
    for i in 0..area.height {
        let char = if i == area.height / 2 {
            if is_active { "◀▶" } else { "┃" }
        } else {
            "│"
        };
        lines.push(Line::from(Span::styled(char, style)));
    }

    let handle = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(handle, area);
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
        animation: &ui_state.animation,
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
        animation: &ui_state.animation,
    };

    ui_state.value_viewer.render(f, area, props);
}

fn render_welcome(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect, keybindings: &KeybindingsConfig) {
    let configs = app_state.connection_manager.get_configs();
    let recent_configs = ui_state
        .recent_connection_ids
        .iter()
        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
        .collect();

    let props = WelcomeScreenProps {
        recent_configs,
        animation: &ui_state.animation,
        keybindings,
    };
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
        animation: &ui_state.animation,
    };

    // Glass effect background
    let glass_bg = Block::default()
        .style(Style::default().bg(Color::Rgb(15, 18, 30)))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme::NEON_PURPLE()));

    f.render_widget(Clear, area);
    f.render_widget(glass_bg, area);
    ui_state.connection_list.render(f, chunks[0], props);

    let instructions = Paragraph::new(Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .fg(theme::BG_DEEP())
                .bg(theme::NEON_CYAN())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" connect ", Style::default().fg(theme::TEXT_SECONDARY())),
        Span::styled(
            " d ",
            Style::default()
                .fg(theme::BG_DEEP())
                .bg(theme::NEON_AMBER())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" delete ", Style::default().fg(theme::TEXT_SECONDARY())),
        Span::styled(
            " Esc ",
            Style::default()
                .fg(theme::BG_DEEP())
                .bg(theme::NEON_PINK())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" close ", Style::default().fg(theme::TEXT_SECONDARY())),
    ]))
    .alignment(Alignment::Center);

    f.render_widget(instructions, chunks[1]);
}

fn render_status_bar(
    f: &mut Frame,
    app_state: &AppState,
    ui_state: &UiState,
    keybindings: &KeybindingsConfig,
    area: Rect,
) {
    // Background
    let bg = Block::default()
        .style(Style::default().bg(theme::BG_PANEL()))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(40, 50, 70)));
    f.render_widget(bg, area);

    let inner_area = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1);

    // Left side: connection status
    let (status_text, status_style, status_icon) =
        if let Some(status) = app_state.connection_manager.get_active_status() {
            match status {
                ConnectionStatus::Connected => {
                    let config_info = app_state
                        .connection_manager
                        .get_active_id()
                        .and_then(|id| app_state.connection_manager.get_config(id))
                        .map(|config| format!("{} ({}:{})", config.name, config.host, config.port))
                        .unwrap_or_else(|| "Connected".to_string());
                    (
                        config_info,
                        theme::status_connected(),
                        theme::INDICATOR_CONNECTED,
                    )
                }
                ConnectionStatus::Connecting => (
                    "Connecting...".to_string(),
                    theme::status_connecting(&ui_state.animation),
                    theme::spinner_pulse(&ui_state.animation),
                ),
                ConnectionStatus::Disconnected => (
                    "Not connected".to_string(),
                    theme::status_disconnected(),
                    theme::INDICATOR_DISCONNECTED,
                ),
                ConnectionStatus::Error(ref msg) => (
                    format!("Error: {}", msg),
                    theme::status_error(),
                    theme::INDICATOR_ERROR,
                ),
            }
        } else {
            (
                "No connection".to_string(),
                theme::status_disconnected(),
                theme::INDICATOR_DISCONNECTED,
            )
        };

    // Right side: keybindings - get from config
    let context = BindingContext::Default;
    let keybinds: Vec<(String, &str)> = vec![
        (
            keybindings
                .get_first_keybinding("navigation.next_panel", context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| "Tab".to_string()),
            "tabs",
        ),
        (
            keybindings
                .get_first_keybinding("connection.palette.toggle", context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| "^P".to_string()),
            "palette",
        ),
        (
            keybindings
                .get_first_keybinding("connection.form.open", context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| "^N".to_string()),
            "new",
        ),
        (
            keybindings
                .get_first_keybinding("help.toggle", context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| "?".to_string()),
            "help",
        ),
        (
            keybindings
                .get_first_keybinding("quit.show", context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| "q".to_string()),
            "quit",
        ),
    ];

    let mut right_spans: Vec<Span> = Vec::new();
    for (key, desc) in keybinds.iter() {
        right_spans.push(Span::styled(key.as_str(), Style::default().fg(theme::NEON_CYAN())));
        right_spans.push(Span::styled(
            format!(" {}  ", desc),
            Style::default().fg(theme::TEXT_DIM()),
        ));
    }

    // Calculate spacing
    let left_content = format!("{} {}", status_icon, status_text);
    let right_content_len: usize = keybinds
        .iter()
        .map(|(k, d)| k.len() + d.len() + 3)
        .sum();
    let padding = inner_area
        .width
        .saturating_sub(left_content.chars().count() as u16 + right_content_len as u16);

    let mut spans = vec![
        Span::styled(format!("{} ", status_icon), status_style),
        Span::styled(status_text, Style::default().fg(theme::TEXT_PRIMARY())),
        Span::raw(" ".repeat(padding as usize)),
    ];
    spans.extend(right_spans);

    let status_line = Paragraph::new(Line::from(spans));
    f.render_widget(status_line, inner_area);
}

fn render_quit_confirmation(f: &mut Frame, _animation: &AnimationState) {
    let area = centered_rect(45, 25, f.area());
    let area = Rect {
        x: area.x,
        y: area.y,
        width: area.width.min(55).max(35),
        height: area.height.min(8).max(6),
    };
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
            Style::default()
                .fg(theme::TEXT_BRIGHT())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " y ",
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::NEON_GREEN())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" yes  ", Style::default().fg(theme::TEXT_SECONDARY())),
            Span::styled(
                " n ",
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::NEON_RED())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" no ", Style::default().fg(theme::TEXT_SECONDARY())),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        "Quit",
                        Style::default()
                            .fg(theme::NEON_AMBER())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ", Style::default()),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::NEON_AMBER())),
        )
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme::BG_SURFACE()));

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
