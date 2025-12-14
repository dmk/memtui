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
    app_state: &AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    action: &Action,
) -> bool {
    match action {
        Action::NextPanel => {
            ui_state.next_panel();
            true
        }
        Action::PrevPanel => {
            ui_state.prev_panel();
            true
        }
        Action::NextItem => {
            if ui_state.show_connection_palette {
                let connections_len = app_state.connection_manager.get_configs().len();
                if connections_len > 0 {
                    ui_state.connection_list.next(connections_len);
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

/// Handle search actions - returns (handled, needs_trigger_search)
pub fn handle_search(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    action: &Action,
) -> Option<(bool, bool)> {
    match action {
        Action::StartSearch => {
            app_state.start_search();
            Some((true, false))
        }
        Action::ClearSearch => {
            app_state.reset_search();
            if !app_state.keys.is_empty() {
                ui_state.key_list.select(Some(0));
                app_state.selected_key_index = Some(0);
                app_state.selected_value = None;
                ui_state.value_viewer.reset_scroll();
            }
            Some((true, false))
        }
        Action::SearchAddChar(c) => {
            if app_state.is_searching {
                app_state.search_query.push(*c);
                Some((true, true)) // needs trigger_search
            } else {
                None
            }
        }
        Action::SearchDeleteChar => {
            if app_state.is_searching {
                app_state.search_query.pop();
                if app_state.search_query.is_empty() {
                    app_state.search_results_local.clear();
                    app_state.search_results_server.clear();
                    Some((true, false))
                } else {
                    Some((true, true)) // needs trigger_search
                }
            } else {
                None
            }
        }
        Action::UpdateSearchQuery(query) => {
            if app_state.is_searching {
                app_state.search_query = query.clone();
                Some((true, true)) // needs trigger_search
            } else {
                None
            }
        }
        _ => None,
    }
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
