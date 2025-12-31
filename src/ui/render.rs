use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use std::time::SystemTime;

use super::components::modal::render_modal;

use super::components::{
    cmdline::CmdLineProps,
    command_suggestions::{command_suggestions, goto_suggestions, CommandSuggestionsProps},
    connection_form::ConnectionFormProps,
    connection_list::ConnectionListProps,
    help::HelpScreenProps,
    key_list::KeyListProps,
    quit_confirmation::QuitConfirmationProps,
    value_viewer::ValueViewerProps,
    warning_message::{MessageKind, WarningMessage, WarningMessageProps},
    welcome::WelcomeScreenProps,
    Component,
};
use super::state::{Panel, TabRegion, UiState};
use super::theme;
use super::widgets::{TabBar, TabItem};
use crate::action::CmdLineMode;
use crate::app::{AppState, ConnectionStatus};
use crate::keybindings::{format_key_for_display, BindingContext, KeybindingsConfig};
use crate::types::ValueType;

/// Main UI rendering function
pub fn render_at(
    f: &mut Frame,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    keybindings: &KeybindingsConfig,
    now: SystemTime,
) {
    // Check if any modal is active and dim the theme for background content
    let modal_active = ui_state.show_connection_palette
        || ui_state.show_connection_form
        || ui_state.show_help
        || ui_state.show_quit_confirmation;

    if modal_active {
        theme::dim_theme(0.7); // 70% dimming for background
    }

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

    let active_id = app_state.connection_manager.get_active_id();
    let capabilities = active_id.and_then(|id| app_state.connection_manager.get_capabilities(id));

    // Check if we should show warning for limited key listing (e.g. memcached)
    let show_limited_keys_warning = capabilities
        .map(|c| c.has_limited_key_listing)
        .unwrap_or(false);

    // Check for temporary connection warning (from CLI connection string)
    let show_temp_connection_warning = ui_state.show_temp_connection_warning;

    // Calculate warning height (can show multiple warnings)
    let warning_height = if show_limited_keys_warning { 1 } else { 0 }
        + if show_temp_connection_warning { 1 } else { 0 };

    // Command line height (1 when active, 0 otherwise)
    let cmdline_height = if app_state.is_cmdline_visible() { 1 } else { 0 };

    // CmdLine suggestions popup height (command/goto modes)
    let (cmdline_suggestions, suggestions_title) = match app_state.cmdline_mode {
        Some(CmdLineMode::Command) => {
            (command_suggestions(&app_state.cmdline_buffer), " Commands ")
        }
        Some(CmdLineMode::Goto) => (
            goto_suggestions(&app_state.cmdline_buffer, &app_state.keys),
            " Goto ",
        ),
        _ => (Vec::new(), ""),
    };
    let suggestions_height = if !cmdline_suggestions.is_empty() {
        // Height = commands (max 6) + 2 for borders
        (cmdline_suggestions.len().min(6) + 2) as u16
    } else {
        0
    };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if show_tabs { 1 } else { 0 }), // Tabs - single line
            Constraint::Length(warning_height),
            Constraint::Min(0),                     // Body
            Constraint::Length(suggestions_height), // Command suggestions popup
            Constraint::Length(cmdline_height),     // Command line (above status bar)
            Constraint::Length(1),                  // Status bar - single line
        ])
        .split(f.area());

    let tab_area = root[0];
    let warning_area = root[1];
    let body_area = root[2];
    let suggestions_area = root[3];
    let cmdline_area = root[4];
    let status_area = root[5];

    // Store body area for resize calculations
    ui_state.last_body_area = Some(body_area);

    if show_tabs {
        render_tabs(f, app_state, ui_state, tab_area);
    }

    // Render warnings
    if warning_height > 0 {
        let warning_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    if show_temp_connection_warning {
                        Constraint::Length(1)
                    } else {
                        Constraint::Length(0)
                    },
                    if show_limited_keys_warning {
                        Constraint::Length(1)
                    } else {
                        Constraint::Length(0)
                    },
                ]
                .iter()
                .filter(|c| !matches!(c, Constraint::Length(0)))
                .cloned()
                .collect::<Vec<_>>(),
            )
            .split(warning_area);

        let mut chunk_idx = 0;

        if show_temp_connection_warning {
            let mut warning_component = WarningMessage::new();
            let warning_props = WarningMessageProps {
                kind: MessageKind::Warning,
                message: "Temporary connection (from CLI) — this connection will not be saved.",
            };
            warning_component.render(f, warning_chunks[chunk_idx], warning_props);
            chunk_idx += 1;
        }

        if show_limited_keys_warning && chunk_idx < warning_chunks.len() {
            let mut warning_component = WarningMessage::new();
            let warning_props = WarningMessageProps {
                kind: MessageKind::Warning,
                message:
                    "This backend has limited key listing capabilities. Keys may not be consistent.",
            };
            warning_component.render(f, warning_chunks[chunk_idx], warning_props);
        }
    }

    if app_state.connection_manager.get_active_id().is_some() {
        render_body(f, app_state, ui_state, body_area, now, keybindings);
    } else {
        ui_state.last_key_area = None;
        ui_state.last_value_area = None;
        render_welcome(f, app_state, ui_state, body_area, keybindings);
    }

    // Render cmdline suggestions popup when in command/goto mode
    if !cmdline_suggestions.is_empty() {
        let props = CommandSuggestionsProps {
            items: &cmdline_suggestions,
            title: suggestions_title,
            selected_index: app_state.suggestions_index,
        };
        ui_state
            .command_suggestions
            .render(f, suggestions_area, props);
    }

    // Render command line above status bar when active
    if app_state.is_cmdline_visible() {
        render_cmdline(f, app_state, ui_state, keybindings, cmdline_area);
    }

    // Status bar is always visible
    render_status_bar(f, app_state, ui_state, keybindings, status_area);

    if ui_state.show_connection_palette {
        render_connection_palette(f, app_state, ui_state, keybindings);
    }

    if ui_state.show_connection_form {
        let area = theme::centered_rect(70, 70, f.area());
        let props = ConnectionFormProps {
            keybindings,
            error: ui_state.form_error.as_deref(),
        };
        ui_state.connection_form.render(f, area, props);
    } else if ui_state.show_help {
        let props = HelpScreenProps { keybindings };
        ui_state.help_screen.render(f, f.area(), props);
    }

    // Quit confirmation should be rendered on top of everything
    if ui_state.show_quit_confirmation {
        let props = QuitConfirmationProps { keybindings };
        ui_state.quit_confirmation.render(f, f.area(), props);
    }

    // Restore theme to normal state after render
    theme::restore_theme();
}

pub fn render(
    f: &mut Frame,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    keybindings: &KeybindingsConfig,
) {
    render_at(f, app_state, ui_state, keybindings, SystemTime::now());
}

fn render_tabs(f: &mut Frame, app_state: &AppState, ui_state: &mut UiState, area: Rect) {
    ui_state.tab_bar_area = Some(area);

    let configs = app_state.connection_manager.get_configs();
    let active_id = app_state.connection_manager.get_active_id();

    // Build tab items from open connections
    let tabs: Vec<TabItem> = configs
        .iter()
        .filter(|c| {
            let status = app_state.connection_manager.get_status(&c.id);
            !matches!(status, ConnectionStatus::Disconnected)
        })
        .map(|config| {
            let is_active = active_id
                .map(|id| id == config.id.as_str())
                .unwrap_or(false);
            TabItem::new(&config.id, &config.name).active(is_active)
        })
        .collect();

    // Use the TabBar widget and store the regions for click handling
    let tab_bar = TabBar::new(&tabs);
    let regions = tab_bar.render(f, area);

    // Convert widget regions to UiState regions
    ui_state.tab_regions = regions
        .into_iter()
        .map(|r| TabRegion {
            id: r.id,
            area: r.area,
        })
        .collect();
}

fn render_body(
    f: &mut Frame,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    area: Rect,
    now: SystemTime,
    keybindings: &KeybindingsConfig,
) {
    // Use the resizable pane split ratio
    let left_percent = ui_state.pane_split.left_percent();
    let right_percent = ui_state.pane_split.right_percent();

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Length(1), // Vertical separator
            Constraint::Percentage(right_percent),
        ])
        .split(area);

    ui_state.last_key_area = Some(body_chunks[0]);
    ui_state.last_value_area = Some(body_chunks[2]);

    // Render vertical separator
    render_separator(f, body_chunks[1]);

    render_keys(f, ui_state, app_state, body_chunks[0], now, keybindings);
    render_value(f, ui_state, app_state, body_chunks[2], now, keybindings);
}

fn render_separator(f: &mut Frame, area: Rect) {
    let top_style = Style::default()
        .fg(theme::BG_PANEL())
        .bg(theme::tab_inactive_bg());
    let rest_style = Style::default().fg(theme::BG_PANEL());
    const BORDER_CHAR: &str = "│";

    let lines: Vec<Line> = std::iter::once(Line::from(Span::styled(BORDER_CHAR, top_style)))
        .chain(std::iter::repeat_n(
            Line::from(Span::styled(BORDER_CHAR, rest_style)),
            (area.height as usize).saturating_sub(1),
        ))
        .collect();
    let separator = Paragraph::new(lines);
    f.render_widget(separator, area);
}

fn render_keys(
    f: &mut Frame,
    ui_state: &mut UiState,
    app_state: &mut AppState,
    area: Rect,
    now: SystemTime,
    keybindings: &KeybindingsConfig,
) {
    app_state.viewport_height = area.height.saturating_sub(2) as usize;

    // CmdLine/search state - only active when in search mode
    let active_search_query = app_state.active_search_query();

    let active_id = app_state.connection_manager.get_active_id();
    let capabilities = active_id.and_then(|id| app_state.connection_manager.get_capabilities(id));

    let props = KeyListProps {
        keys: &app_state.keys,
        total_count: app_state.total_key_count,
        is_loading: app_state.is_loading_keys,
        active_search_query,
        is_active: ui_state.active_panel == Panel::Keys,
        capabilities,
        is_searching: app_state.is_searching(),
        search_results_local: &app_state.search_results_local,
        search_results_server: &app_state.search_results_server,
        is_server_searching: app_state.is_server_searching,
        search_selection_index: app_state.search_selection_index,
        animation: &ui_state.animation,
        search_match_positions: &app_state.search_match_positions,
        now,
        keybindings,
    };

    ui_state.key_list.render(f, area, props);
}

fn render_value(
    f: &mut Frame,
    ui_state: &mut UiState,
    app_state: &AppState,
    area: Rect,
    now: SystemTime,
    keybindings: &KeybindingsConfig,
) {
    let command_result = app_state.command_result.as_ref();
    let (selected_value, selected_key_type, selected_key_name, selected_key, command_backend_type) =
        if let Some(result) = command_result {
            (
                Some(&result.value),
                Some(ValueType::String),
                Some(result.title.as_str()),
                None,
                result.backend_type,
            )
        } else {
            let selected_key = app_state
                .selected_key_index
                .and_then(|idx| app_state.keys.get(idx))
                .and_then(|k| k.as_ref());

            (
                app_state.selected_value.as_ref(),
                selected_key.map(|k| k.value_type),
                selected_key.map(|k| k.name.as_str()),
                selected_key,
                None,
            )
        };

    let active_id = app_state.connection_manager.get_active_id();
    let capabilities = active_id.and_then(|id| app_state.connection_manager.get_capabilities(id));

    let props = ValueViewerProps {
        selected_value,
        selected_key_type,
        selected_key_name,
        selected_key,
        error_message: app_state.error_message.as_ref(),
        json_formatter: &app_state.json_formatter,
        text_formatter: &app_state.text_formatter,
        is_active: ui_state.active_panel == Panel::Value,
        capabilities,
        animation: &ui_state.animation,
        now,
        keybindings,
        command_backend_type,
    };

    ui_state.value_viewer.render(f, area, props);
}

fn render_welcome(
    f: &mut Frame,
    app_state: &AppState,
    ui_state: &mut UiState,
    area: Rect,
    keybindings: &KeybindingsConfig,
) {
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

fn render_connection_palette(
    f: &mut Frame,
    app_state: &AppState,
    ui_state: &mut UiState,
    keybindings: &KeybindingsConfig,
) {
    use theme::raw;

    let area = centered_rect(60, 70, f.area());

    // Render modal (uses raw colors internally)
    let inner = render_modal(f, area, "Connections");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(inner);

    ui_state.connection_palette_area = Some(chunks[0]);

    let props = ConnectionListProps {
        configs: app_state.connection_manager.get_configs(),
        active_id: app_state.connection_manager.get_active_id(),
        statuses: app_state.connection_manager.get_statuses(),
        is_active: true,
        animation: &ui_state.animation,
        keybindings,
    };

    ui_state.connection_list.render(f, chunks[0], props);

    // Use raw colors for modal content
    let instructions = Paragraph::new(Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .fg(raw::BG_DEEP())
                .bg(raw::NEON_CYAN())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" connect ", Style::default().fg(raw::TEXT_SECONDARY())),
        Span::styled(
            " d ",
            Style::default()
                .fg(raw::BG_DEEP())
                .bg(raw::NEON_AMBER())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" delete ", Style::default().fg(raw::TEXT_SECONDARY())),
        Span::styled(
            " Esc ",
            Style::default()
                .fg(raw::BG_DEEP())
                .bg(raw::NEON_PINK())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" close ", Style::default().fg(raw::TEXT_SECONDARY())),
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
    // Get selected key name for left side
    let selected_key_name = app_state
        .selected_key_index
        .and_then(|idx| app_state.keys.get(idx))
        .and_then(|k| k.as_ref())
        .map(|k| k.name.as_str());

    // Get connection info for right side
    let (conn_text, conn_style, conn_icon) =
        if let Some(status) = app_state.connection_manager.get_active_status() {
            match status {
                ConnectionStatus::Connected => {
                    let config_info = app_state
                        .connection_manager
                        .get_active_id()
                        .and_then(|id| app_state.connection_manager.get_config(id))
                        .map(|config| format!("{}:{}", config.host, config.port))
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
                ConnectionStatus::Error(ref msg) => {
                    // Truncate error messages
                    let short_msg = if msg.len() > 20 {
                        format!("{}...", &msg[..17])
                    } else {
                        msg.clone()
                    };
                    (short_msg, theme::status_error(), theme::INDICATOR_ERROR)
                }
            }
        } else {
            (
                "No connection".to_string(),
                theme::status_disconnected(),
                theme::INDICATOR_DISCONNECTED,
            )
        };

    // Build keybindings string (center section - always at fixed center position)
    let context = BindingContext::Default;
    let help_key = keybindings
        .get_first_keybinding("help.toggle", context)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "?".to_string());
    let quit_key = keybindings
        .get_first_keybinding("quit.show", context)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "Q".to_string());

    let keybinds_str = format!("{} help  {} quit", help_key, quit_key);
    let keybinds_len = keybinds_str.chars().count();

    // Connection info string
    let conn_str = format!("{} {}", conn_icon, conn_text);
    let conn_len = conn_str.chars().count();

    // Use fixed 3-section layout: [left 35%] [center 30%] [right 35%]
    // This keeps the help text at a stable position
    let total_width = area.width as usize;
    let left_width = total_width * 35 / 100;
    let center_width = total_width * 30 / 100;
    let right_width = total_width.saturating_sub(left_width + center_width);

    // Left section: error or key name (truncate if needed)
    let (key_display, key_style) = if let Some(error) = app_state.error_message.as_ref() {
        let max_len = left_width.saturating_sub(2);
        let display = if error.chars().count() > max_len {
            let truncated: String = error.chars().take(max_len.saturating_sub(1)).collect();
            format!(" ✕ {}…", truncated)
        } else {
            format!(" ✕ {}", error)
        };
        (display, Style::default().fg(theme::NEON_RED()))
    } else if let Some(name) = selected_key_name {
        let max_len = left_width.saturating_sub(2); // leave some margin
        let display = if name.chars().count() > max_len {
            let truncated: String = name.chars().take(max_len.saturating_sub(1)).collect();
            format!(" {}…", truncated)
        } else {
            format!(" {}", name)
        };
        (display, Style::default().fg(theme::TEXT_PRIMARY()))
    } else {
        (String::new(), Style::default().fg(theme::TEXT_PRIMARY()))
    };
    let left_len = key_display.chars().count();
    let left_padding = left_width.saturating_sub(left_len);

    // Center section: keybindings (centered within center section)
    let center_left_pad = center_width.saturating_sub(keybinds_len) / 2;
    let center_right_pad = center_width.saturating_sub(keybinds_len + center_left_pad);

    // Right section: connection info (right-aligned)
    let right_padding = right_width.saturating_sub(conn_len + 1);

    // Build the status line with fixed positions
    let spans = vec![
        // Left section
        Span::styled(&key_display, key_style),
        Span::raw(" ".repeat(left_padding)),
        // Center section
        Span::raw(" ".repeat(center_left_pad)),
        Span::styled(&keybinds_str, Style::default().fg(theme::TEXT_DIM())),
        Span::raw(" ".repeat(center_right_pad)),
        // Right section
        Span::raw(" ".repeat(right_padding)),
        Span::styled(conn_icon, conn_style),
        Span::styled(
            format!(" {}", conn_text),
            Style::default().fg(theme::TEXT_DIM()),
        ),
        Span::raw(" "),
    ];

    let status_line = Paragraph::new(Line::from(spans)).style(theme::util_bg());
    f.render_widget(status_line, area);
}

fn render_cmdline(
    f: &mut Frame,
    app_state: &AppState,
    ui_state: &mut UiState,
    keybindings: &KeybindingsConfig,
    area: Rect,
) {
    // Calculate search result count for display
    let search_result_count = if app_state.is_searching() && !app_state.cmdline_buffer.is_empty() {
        Some(app_state.search_results_local.len() + app_state.search_results_server.len())
    } else {
        None
    };

    let props = CmdLineProps {
        keybindings,
        mode: app_state.cmdline_mode,
        is_active: app_state.cmdline_active,
        buffer: &app_state.cmdline_buffer,
        cursor: app_state.cmdline_cursor,
        search_result_count,
        is_server_searching: app_state.is_server_searching,
    };

    ui_state.cmdline.render(f, area, props);
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
