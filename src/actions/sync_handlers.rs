//! Synchronous action handlers (UI state changes, navigation)
//!
//! These handlers update UI state directly without spawning async tasks.

use crate::action::Action;
use crate::app::AppState;
use crate::config::Config;
use crate::ui::{Panel, UiState};
use crate::userdata;
use tokio::sync::mpsc;

/// Handle UI state toggle actions
pub fn handle_ui_toggle(ui_state: &mut UiState, app_state: &AppState, action: &Action) -> bool {
    match action {
        Action::ShowQuitConfirmation => {
            ui_state.show_quit_confirmation = true;
            true
        }
        Action::CancelQuit => {
            ui_state.show_quit_confirmation = false;
            true
        }
        Action::ToggleHelp => {
            ui_state.show_help = !ui_state.show_help;
            true
        }
        Action::OpenConnectionForm => {
            ui_state.open_connection_form();
            true
        }
        Action::CloseConnectionForm => {
            ui_state.close_connection_form();
            true
        }
        Action::OpenConnectionPalette => {
            ui_state.open_connection_palette();
            let configs = app_state.connection_manager.get_configs();
            if configs.is_empty() {
                ui_state.connection_list.state.select(None);
            } else if let Some(active_id) = app_state.connection_manager.get_active_id() {
                if let Some(idx) = configs.iter().position(|cfg| cfg.id == active_id) {
                    ui_state.connection_list.state.select(Some(idx));
                } else {
                    ui_state.connection_list.state.select(Some(0));
                }
            } else {
                ui_state.connection_list.state.select(Some(0));
            }
            true
        }
        Action::CloseConnectionPalette => {
            ui_state.close_connection_palette();
            true
        }
        _ => false,
    }
}

/// Handle navigation actions (NextItem, PrevItem, NextPanel, etc.)
pub fn handle_navigation(
    ui_state: &mut UiState,
    app_state: &mut AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    action: &Action,
) -> bool {
    match action {
        Action::NextPanel => {
            // Exit cmdline mode when switching panels
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.next_panel();
            true
        }
        Action::PrevPanel => {
            // Exit cmdline mode when switching panels
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.prev_panel();
            true
        }
        Action::NextItem => {
            if ui_state.show_connection_palette {
                let connections_len = app_state.connection_manager.get_configs().len();
                if connections_len > 0 {
                    ui_state.connection_list.next(connections_len);
                }
            } else if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
                // Navigate through search results (only in search mode)
                let total_count = app_state.search_results_local.len();
                if total_count > 0 {
                    let current_idx = app_state.search_selection_index.unwrap_or(0);
                    let new_idx = if current_idx >= total_count - 1 {
                        0
                    } else {
                        current_idx + 1
                    };
                    let _ = action_tx.send(Action::SetCmdLineSelectionIndex(Some(new_idx)));
                    if let Some(&key_idx) = app_state.search_results_local.get(new_idx) {
                        let _ = action_tx.send(Action::SelectKey(key_idx));
                    }
                }
            } else {
                let keys_len = app_state
                    .total_key_count
                    .map(|t| t as usize)
                    .unwrap_or(app_state.keys.len());

                if ui_state.next_item(keys_len) && ui_state.active_panel == Panel::Keys {
                    if let Some(idx) = ui_state.key_list.state.selected() {
                        let _ = action_tx.send(Action::SelectKey(idx));
                    }
                }
            }
            true
        }
        Action::PrevItem => {
            if ui_state.show_connection_palette {
                let connections_len = app_state.connection_manager.get_configs().len();
                if connections_len > 0 {
                    ui_state.connection_list.prev(connections_len);
                }
            } else if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
                // Navigate through search results (only in search mode)
                let total_count = app_state.search_results_local.len();
                if total_count > 0 {
                    let current_idx = app_state.search_selection_index.unwrap_or(0);
                    let new_idx = if current_idx == 0 {
                        total_count - 1
                    } else {
                        current_idx - 1
                    };
                    let _ = action_tx.send(Action::SetCmdLineSelectionIndex(Some(new_idx)));
                    if let Some(&key_idx) = app_state.search_results_local.get(new_idx) {
                        let _ = action_tx.send(Action::SelectKey(key_idx));
                    }
                }
            } else {
                let keys_len = app_state
                    .total_key_count
                    .map(|t| t as usize)
                    .unwrap_or(app_state.keys.len());

                if ui_state.previous_item(keys_len) && ui_state.active_panel == Panel::Keys {
                    if let Some(idx) = ui_state.key_list.state.selected() {
                        let _ = action_tx.send(Action::SelectKey(idx));
                    }
                }
            }
            true
        }
        _ => false,
    }
}

/// Handle direct UI state actions (from mouse events, etc.)
pub fn handle_ui_state(ui_state: &mut UiState, app_state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::FocusPanel(panel) => {
            // Exit cmdline mode when switching panels
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.active_panel = *panel;
            true
        }
        Action::SetPaneRatio(ratio) => {
            ui_state.pane_split.ratio =
                ratio.clamp(ui_state.pane_split.min, ui_state.pane_split.max);
            true
        }
        Action::SelectConnectionIndex(idx) => {
            ui_state.connection_list.state.select(Some(*idx));
            true
        }
        Action::SelectWelcomeIndex(idx) => {
            ui_state.welcome_screen.state.select(Some(*idx));
            true
        }
        Action::SetCmdLineSelectionIndex(idx) => {
            app_state.search_selection_index = *idx;
            true
        }
        Action::ResetKeySelection => {
            ui_state.key_list.select(None);
            app_state.reset_pagination();
            app_state.error_message = None;
            true
        }
        Action::ExitCmdLineMode => {
            app_state.deactivate_cmdline();
            true
        }
        Action::StartResize => {
            ui_state.start_resize();
            true
        }
        Action::EndResize => {
            ui_state.end_resize();
            true
        }
        _ => false,
    }
}

/// Handle connection form actions
pub fn handle_connection_form(
    ui_state: &mut UiState,
    app_state: &mut AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    config: &Config,
    action: &Action,
) -> bool {
    match action {
        Action::ConnectionFormNextField => {
            ui_state.connection_form.next_field();
            true
        }
        Action::ConnectionFormPrevField => {
            ui_state.connection_form.prev_field();
            true
        }
        Action::ConnectionFormAddChar(c) => {
            ui_state.connection_form.insert_char(*c);
            true
        }
        Action::ConnectionFormDeleteChar => {
            ui_state.connection_form.delete_char();
            true
        }
        Action::ConnectionFormDeleteForward => {
            ui_state.connection_form.delete_forward();
            true
        }
        Action::ConnectionFormDeleteWordBack => {
            ui_state.connection_form.delete_word_back();
            true
        }
        Action::ConnectionFormDeleteWordForward => {
            ui_state.connection_form.delete_word_forward();
            true
        }
        Action::ConnectionFormMoveLeft => {
            ui_state.connection_form.move_left();
            true
        }
        Action::ConnectionFormMoveRight => {
            ui_state.connection_form.move_right();
            true
        }
        Action::ConnectionFormMoveWordLeft => {
            ui_state.connection_form.move_word_left();
            true
        }
        Action::ConnectionFormMoveWordRight => {
            ui_state.connection_form.move_word_right();
            true
        }
        Action::ConnectionFormMoveStart => {
            ui_state.connection_form.move_start();
            true
        }
        Action::ConnectionFormMoveEnd => {
            ui_state.connection_form.move_end();
            true
        }
        Action::ConnectionFormNextBackendType => {
            ui_state.connection_form.next_backend_type();
            true
        }
        Action::ConnectionFormPrevBackendType => {
            ui_state.connection_form.prev_backend_type();
            true
        }
        Action::Enter if ui_state.show_connection_form => {
            match ui_state
                .connection_form
                .to_config(config.connection.default_timeout)
            {
                Ok(conn_config) => {
                    let _ = action_tx.send(Action::SubmitConnectionForm(conn_config));
                }
                Err(e) => ui_state.set_form_error(e),
            }
            true
        }
        Action::SubmitConnectionForm(conn_config) => {
            app_state
                .connection_manager
                .add_connection(conn_config.clone());
            let all_configs = app_state.connection_manager.get_all_configs();
            let _ = userdata::save_connections(&all_configs);
            ui_state.close_connection_form();
            let _ = action_tx.send(Action::FocusConnection(conn_config.id.clone()));
            true
        }
        _ => false,
    }
}

/// Handle connection management actions
pub fn handle_connection_management(
    ui_state: &mut UiState,
    app_state: &mut AppState,
    action: &Action,
) -> bool {
    match action {
        Action::DeleteConnection(id) => {
            app_state.connection_manager.remove_config(id);
            let all_configs = app_state.connection_manager.get_all_configs();
            let _ = userdata::save_connections(&all_configs);
            if let Ok(ids) = userdata::remove_recent_connection_id(id) {
                ui_state.recent_connection_ids = ids;
            }

            let remaining = app_state.connection_manager.get_configs();
            if remaining.is_empty() {
                ui_state.connection_list.state.select(None);
                ui_state.close_connection_palette();
            } else {
                let current = ui_state
                    .connection_list
                    .state
                    .selected()
                    .unwrap_or(0)
                    .min(remaining.len().saturating_sub(1));
                ui_state.connection_list.state.select(Some(current));
            }
            true
        }
        _ => false,
    }
}

/// Handle cmdline actions - returns (handled, needs_trigger_search)
pub fn handle_cmdline(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    action: &Action,
) -> Option<(bool, bool)> {
    use crate::action::CmdLineMode;

    match action {
        Action::SetCmdLineMode(mode) => {
            if *mode == CmdLineMode::Search {
                app_state.command_result = None;
            }
            if app_state.cmdline_mode == Some(*mode) {
                app_state.activate_cmdline();
            } else {
                app_state.start_cmdline(*mode);
            }
            let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search)
                && !app_state.cmdline_buffer.is_empty();
            Some((true, needs_search))
        }
        Action::CmdLineClear => {
            let was_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
            if was_search {
                if app_state.cmdline_buffer.is_empty() {
                    app_state.reset_cmdline();
                } else {
                    app_state.cmdline_buffer.clear();
                    app_state.cmdline_cursor = 0;
                    app_state.cmdline_active = false;
                    app_state.search_token = app_state.search_token.wrapping_add(1);
                    app_state.clear_search_state();
                }
                if !app_state.keys.is_empty() {
                    ui_state.key_list.select(Some(0));
                    app_state.selected_key_index = Some(0);
                    app_state.selected_value = None;
                    ui_state.value_viewer.reset_scroll();
                }
            } else {
                app_state.reset_cmdline();
            }
            Some((true, false))
        }
        Action::CmdLineHide => {
            if app_state.cmdline_mode != Some(CmdLineMode::Search) {
                app_state.reset_cmdline();
                Some((true, false))
            } else {
                Some((false, false))
            }
        }
        Action::CmdLineSetCommandResult {
            command,
            output,
            status,
        } => {
            app_state.command_result = Some(crate::app::CommandResult::new(
                command.clone(),
                output.clone(),
                *status,
            ));
            app_state.selected_key_index = None;
            app_state.selected_value = None;
            app_state.error_message = None;
            ui_state.key_list.select(None);
            ui_state.value_viewer.reset_scroll();
            Some((true, false))
        }
        Action::CmdLineClearCommandResult => {
            app_state.command_result = None;
            Some((true, false))
        }
        Action::CmdLineAddChar(c) => {
            if app_state.is_cmdline_active() {
                // Insert at cursor position (cursor is char index, need byte index for insert)
                let byte_idx =
                    char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                app_state.cmdline_buffer.insert(byte_idx, *c);
                app_state.cmdline_cursor += 1;
                // Only trigger search in search mode
                let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                Some((true, needs_search))
            } else {
                None
            }
        }
        Action::CmdLineDeleteChar => {
            if app_state.is_cmdline_active() && app_state.cmdline_cursor > 0 {
                app_state.cmdline_cursor -= 1;
                // Convert char index to byte index for remove
                let byte_idx =
                    char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                app_state.cmdline_buffer.remove(byte_idx);
                if app_state.cmdline_buffer.is_empty() {
                    if app_state.cmdline_mode == Some(CmdLineMode::Search) {
                        app_state.search_token = app_state.search_token.wrapping_add(1);
                        app_state.clear_search_state();
                    }
                    Some((true, false))
                } else {
                    let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                    Some((true, needs_search))
                }
            } else {
                Some((true, false))
            }
        }
        Action::CmdLineDeleteForward => {
            let char_len = app_state.cmdline_buffer.chars().count();
            if app_state.is_cmdline_active() && app_state.cmdline_cursor < char_len {
                // Convert char index to byte index for remove
                let byte_idx =
                    char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                app_state.cmdline_buffer.remove(byte_idx);
                let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                Some((true, needs_search))
            } else {
                Some((true, false))
            }
        }
        Action::CmdLineMoveLeft => {
            if app_state.is_cmdline_active() && app_state.cmdline_cursor > 0 {
                app_state.cmdline_cursor -= 1;
            }
            Some((true, false))
        }
        Action::CmdLineMoveRight => {
            let char_len = app_state.cmdline_buffer.chars().count();
            if app_state.is_cmdline_active() && app_state.cmdline_cursor < char_len {
                app_state.cmdline_cursor += 1;
            }
            Some((true, false))
        }
        Action::CmdLineMoveStart => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor = 0;
            }
            Some((true, false))
        }
        Action::CmdLineMoveEnd => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor = app_state.cmdline_buffer.chars().count();
            }
            Some((true, false))
        }
        Action::CmdLineMoveWordLeft => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor =
                    find_word_boundary_left(&app_state.cmdline_buffer, app_state.cmdline_cursor);
            }
            Some((true, false))
        }
        Action::CmdLineMoveWordRight => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor =
                    find_word_boundary_right(&app_state.cmdline_buffer, app_state.cmdline_cursor);
            }
            Some((true, false))
        }
        Action::CmdLineDeleteWordBack => {
            if app_state.is_cmdline_active() && app_state.cmdline_cursor > 0 {
                let new_cursor =
                    find_word_boundary_left(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                // Convert char indices to byte indices for drain
                let start_byte = char_to_byte_index(&app_state.cmdline_buffer, new_cursor);
                let end_byte =
                    char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                app_state.cmdline_buffer.drain(start_byte..end_byte);
                app_state.cmdline_cursor = new_cursor;
                let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                Some((true, needs_search))
            } else {
                Some((true, false))
            }
        }
        Action::CmdLineDeleteWordForward => {
            let char_len = app_state.cmdline_buffer.chars().count();
            if app_state.is_cmdline_active() && app_state.cmdline_cursor < char_len {
                let end =
                    find_word_boundary_right(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                // Convert char indices to byte indices for drain
                let start_byte =
                    char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
                let end_byte = char_to_byte_index(&app_state.cmdline_buffer, end);
                app_state.cmdline_buffer.drain(start_byte..end_byte);
                let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                Some((true, needs_search))
            } else {
                Some((true, false))
            }
        }
        Action::CmdLineUpdateQuery(query) => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_buffer = query.clone();
                app_state.cmdline_cursor = query.chars().count();
                let needs_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
                Some((true, needs_search))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert a character index to a byte index in the string
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

/// Find word boundary to the left of cursor (cursor is char index)
fn find_word_boundary_left(s: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut pos = cursor.min(chars.len());
    // Skip trailing whitespace
    while pos > 0 && chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    // Find start of word
    while pos > 0 && !chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    pos
}

/// Find word boundary to the right of cursor (cursor is char index)
fn find_word_boundary_right(s: &str, cursor: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut pos = cursor.min(len);
    // Skip current word
    while pos < len && !chars[pos].is_whitespace() {
        pos += 1;
    }
    // Skip whitespace
    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }
    pos
}

/// Handle Enter action for connection palette
pub fn handle_enter_connection_palette(
    ui_state: &mut UiState,
    app_state: &AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
) -> bool {
    if !ui_state.show_connection_palette {
        return false;
    }

    if let Some(idx) = ui_state.connection_list.state.selected() {
        let configs = app_state.connection_manager.get_configs();
        if let Some(config) = configs.get(idx) {
            let _ = action_tx.send(Action::FocusConnection(config.id.clone()));
        }
    }
    ui_state.close_connection_palette();
    true
}

/// Handle error actions
pub fn handle_error(app_state: &mut AppState, error: String) {
    app_state.error_message = Some(error);
}

/// Handle DidFailScanKeys
pub fn handle_did_fail_scan_keys(app_state: &mut AppState, error: String) {
    app_state.error_message = Some(error);
    app_state.is_loading_keys = false;
}

/// Handle DidFailLoadValue
pub fn handle_did_fail_load_value(app_state: &mut AppState, error: String) {
    app_state.error_message = Some(format!("Error loading value: {}", error));
    app_state.selected_value = None;
}

/// Handle value viewer actions
pub fn handle_value_viewer(ui_state: &mut UiState, action: &Action) -> bool {
    match action {
        Action::ValueViewerScrollUp => {
            ui_state.value_viewer.scroll_up();
            true
        }
        Action::ValueViewerScrollDown => {
            ui_state.value_viewer.scroll_down();
            true
        }
        Action::ValueViewerScrollBy(delta) => {
            ui_state.value_viewer.scroll_by(*delta as isize);
            true
        }
        _ => false,
    }
}

/// Handle connection list (palette) actions
pub fn handle_connection_list(
    ui_state: &mut UiState,
    app_state: &AppState,
    action: &Action,
) -> bool {
    match action {
        Action::ConnectionListNext => {
            let len = app_state.connection_manager.get_configs().len();
            if len > 0 {
                ui_state.connection_list.next(len);
            }
            true
        }
        Action::ConnectionListPrev => {
            let len = app_state.connection_manager.get_configs().len();
            if len > 0 {
                ui_state.connection_list.prev(len);
            }
            true
        }
        _ => false,
    }
}

/// Handle welcome screen actions
pub fn handle_welcome(ui_state: &mut UiState, recent_count: usize, action: &Action) -> bool {
    match action {
        Action::WelcomeNextItem => {
            ui_state.welcome_screen.next(recent_count);
            true
        }
        Action::WelcomePrevItem => {
            ui_state.welcome_screen.prev(recent_count);
            true
        }
        _ => false,
    }
}

/// Handle SetActiveConnection action
pub fn handle_set_active_connection(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    config: &crate::config::Config,
    id: &str,
) -> bool {
    if !app_state.connection_manager.set_active(id) {
        return false;
    }
    if let Ok(ids) = userdata::record_recent_connection_id_with_config(id, config) {
        ui_state.recent_connection_ids = ids;
    }
    true
}
