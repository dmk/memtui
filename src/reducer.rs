//! Pure reducer that processes actions and returns state changes + effects.
//!
//! The reducer is the central dispatcher for all actions. It mutates state
//! synchronously and returns effects that need to be handled asynchronously.

use std::sync::Arc;
use tokio::sync::RwLock;
use tui_dispatch::DispatchResult;

use crate::action::{Action, CmdLineMode};
use crate::app::{AppState, CommandResult, ConnectionStatus};
use crate::backend::{Backend, BackendCapabilities, CommandStatus};
use crate::config::Config;
use crate::effect::Effect;
use crate::types::KeyMetadata;
use crate::ui::components::command_suggestions::{command_suggestions, goto_suggestions};
use crate::ui::{Panel, UiState};
use crate::userdata;

/// Context passed to the reducer for handling actions.
pub struct ReducerContext<'a> {
    pub config: &'a Config,
}

/// Process an action and return state changes + effects.
///
/// Returns a `DispatchResult<Effect>` containing:
/// - `changed`: whether the state was modified (triggers re-render)
/// - `effects`: list of side effects to handle asynchronously
pub fn reduce(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    action: Action,
    ctx: &ReducerContext<'_>,
) -> DispatchResult<Effect> {
    match action {
        // =========================================================================
        // Core lifecycle
        // =========================================================================
        Action::Tick => DispatchResult::changed(),
        Action::Render => DispatchResult::changed(),
        Action::Quit => DispatchResult::unchanged(),
        Action::ConfirmQuit => DispatchResult::unchanged(),
        Action::Resize(_, _) => DispatchResult::changed(),

        // =========================================================================
        // UI toggles
        // =========================================================================
        Action::ShowQuitConfirmation => {
            ui_state.show_quit_confirmation = true;
            DispatchResult::changed()
        }
        Action::CancelQuit => {
            ui_state.show_quit_confirmation = false;
            DispatchResult::changed()
        }
        Action::ToggleHelp => {
            ui_state.show_help = !ui_state.show_help;
            DispatchResult::changed()
        }
        Action::OpenConnectionForm => {
            ui_state.open_connection_form();
            DispatchResult::changed()
        }
        Action::CloseConnectionForm => {
            ui_state.close_connection_form();
            DispatchResult::changed()
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
            DispatchResult::changed()
        }
        Action::CloseConnectionPalette => {
            ui_state.close_connection_palette();
            DispatchResult::changed()
        }

        // =========================================================================
        // Navigation
        // =========================================================================
        Action::NextPanel => {
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.next_panel();
            DispatchResult::changed()
        }
        Action::PrevPanel => {
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.prev_panel();
            DispatchResult::changed()
        }
        Action::NextItem => handle_next_item(app_state, ui_state),
        Action::PrevItem => handle_prev_item(app_state, ui_state),
        Action::PageDown => handle_page_down(app_state, ui_state),
        Action::PageUp => handle_page_up(app_state, ui_state),
        Action::Home => handle_home(app_state, ui_state),
        Action::End => handle_end(app_state, ui_state),
        Action::Enter => {
            // Context-dependent - handled in main.rs for complex cases
            DispatchResult::changed()
        }
        Action::Escape => DispatchResult::changed(),

        // =========================================================================
        // UI state changes
        // =========================================================================
        Action::FocusPanel(panel) => {
            if app_state.is_cmdline_active() {
                app_state.deactivate_cmdline();
            }
            ui_state.active_panel = panel;
            DispatchResult::changed()
        }
        Action::SetPaneRatio(ratio) => {
            ui_state.pane_split.ratio =
                ratio.clamp(ui_state.pane_split.min, ui_state.pane_split.max);
            DispatchResult::changed()
        }
        Action::SelectConnectionIndex(idx) => {
            ui_state.connection_list.state.select(Some(idx));
            DispatchResult::changed()
        }
        Action::SelectWelcomeIndex(idx) => {
            ui_state.welcome_screen.state.select(Some(idx));
            DispatchResult::changed()
        }
        Action::SetCmdLineSelectionIndex(idx) => {
            app_state.search_selection_index = idx;
            DispatchResult::changed()
        }
        Action::ResetKeySelection => {
            ui_state.key_list.select(None);
            app_state.reset_pagination();
            app_state.error_message = None;
            DispatchResult::changed()
        }
        Action::ExitCmdLineMode => {
            app_state.deactivate_cmdline();
            DispatchResult::changed()
        }
        Action::StartResize => {
            ui_state.start_resize();
            DispatchResult::changed()
        }
        Action::EndResize => {
            ui_state.end_resize();
            DispatchResult::changed()
        }

        // =========================================================================
        // Connection form
        // =========================================================================
        Action::ConnectionFormNextField => {
            ui_state.connection_form.next_field();
            DispatchResult::changed()
        }
        Action::ConnectionFormPrevField => {
            ui_state.connection_form.prev_field();
            DispatchResult::changed()
        }
        Action::ConnectionFormAddChar(c) => {
            ui_state.connection_form.insert_char(c);
            DispatchResult::changed()
        }
        Action::ConnectionFormDeleteChar => {
            ui_state.connection_form.delete_char();
            DispatchResult::changed()
        }
        Action::ConnectionFormDeleteForward => {
            ui_state.connection_form.delete_forward();
            DispatchResult::changed()
        }
        Action::ConnectionFormDeleteWordBack => {
            ui_state.connection_form.delete_word_back();
            DispatchResult::changed()
        }
        Action::ConnectionFormDeleteWordForward => {
            ui_state.connection_form.delete_word_forward();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveLeft => {
            ui_state.connection_form.move_left();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveRight => {
            ui_state.connection_form.move_right();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveWordLeft => {
            ui_state.connection_form.move_word_left();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveWordRight => {
            ui_state.connection_form.move_word_right();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveStart => {
            ui_state.connection_form.move_start();
            DispatchResult::changed()
        }
        Action::ConnectionFormMoveEnd => {
            ui_state.connection_form.move_end();
            DispatchResult::changed()
        }
        Action::ConnectionFormNextBackendType => {
            ui_state.connection_form.next_backend_type();
            DispatchResult::changed()
        }
        Action::ConnectionFormPrevBackendType => {
            ui_state.connection_form.prev_backend_type();
            DispatchResult::changed()
        }
        Action::SubmitConnectionForm(conn_config) => {
            app_state
                .connection_manager
                .add_connection(conn_config.clone());
            let all_configs = app_state.connection_manager.get_all_configs();
            let _ = userdata::save_connections(&all_configs);
            ui_state.close_connection_form();
            DispatchResult::changed()
        }

        // =========================================================================
        // Value viewer
        // =========================================================================
        Action::CycleValueViewMode => {
            // Handled in main.rs due to complex conditions
            DispatchResult::unchanged()
        }
        Action::ValueViewerScrollUp => {
            ui_state.value_viewer.scroll_up();
            DispatchResult::changed()
        }
        Action::ValueViewerScrollDown => {
            ui_state.value_viewer.scroll_down();
            DispatchResult::changed()
        }
        Action::ValueViewerScrollBy(delta) => {
            ui_state.value_viewer.scroll_by(delta as isize);
            DispatchResult::changed()
        }

        // =========================================================================
        // Connection list (palette)
        // =========================================================================
        Action::ConnectionListNext => {
            let len = app_state.connection_manager.get_configs().len();
            if len > 0 {
                ui_state.connection_list.next(len);
            }
            DispatchResult::changed()
        }
        Action::ConnectionListPrev => {
            let len = app_state.connection_manager.get_configs().len();
            if len > 0 {
                ui_state.connection_list.prev(len);
            }
            DispatchResult::changed()
        }

        // =========================================================================
        // Welcome screen
        // =========================================================================
        Action::WelcomeNextItem => {
            let configs = app_state.connection_manager.get_configs();
            let recent_count = ui_state
                .recent_connection_ids
                .iter()
                .filter(|id| configs.iter().any(|c| &c.id == *id))
                .count();
            ui_state.welcome_screen.next(recent_count);
            DispatchResult::changed()
        }
        Action::WelcomePrevItem => {
            let configs = app_state.connection_manager.get_configs();
            let recent_count = ui_state
                .recent_connection_ids
                .iter()
                .filter(|id| configs.iter().any(|c| &c.id == *id))
                .count();
            ui_state.welcome_screen.prev(recent_count);
            DispatchResult::changed()
        }

        // =========================================================================
        // Connection tabs
        // =========================================================================
        Action::NextConnectionTab | Action::PrevConnectionTab => {
            // Handled in main.rs due to complex focus_connection logic
            DispatchResult::changed()
        }

        // =========================================================================
        // Connection management
        // =========================================================================
        Action::SetActiveConnection(id) => {
            if app_state.connection_manager.set_active(&id) {
                if let Ok(ids) = userdata::record_recent_connection_id_with_config(&id, ctx.config)
                {
                    ui_state.recent_connection_ids = ids;
                }
                DispatchResult::changed()
            } else {
                DispatchResult::unchanged()
            }
        }
        Action::FocusConnection(_id) => {
            // Handled in main.rs due to complex logic
            DispatchResult::changed()
        }
        Action::DeleteConnection(id) => {
            app_state.connection_manager.remove_config(&id);
            let all_configs = app_state.connection_manager.get_all_configs();
            let _ = userdata::save_connections(&all_configs);
            if let Ok(ids) = userdata::remove_recent_connection_id(&id) {
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
            DispatchResult::changed()
        }

        // =========================================================================
        // Connection actions (emit effects)
        // =========================================================================
        Action::Connect(id) => {
            if let Some(config) = app_state.connection_manager.get_config(&id).cloned() {
                app_state
                    .connection_manager
                    .set_status(&id, ConnectionStatus::Connecting);
                DispatchResult::changed_with(Effect::Connect { id, config })
            } else {
                DispatchResult::unchanged()
            }
        }
        Action::Disconnect(id) => DispatchResult::changed_with(Effect::Disconnect { id }),

        // =========================================================================
        // Data loading (emit effects)
        // =========================================================================
        Action::LoadKeys => {
            app_state.is_loading_keys = true;
            app_state.reset_pagination();
            if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
                let chunk_size = app_state.keys_per_chunk;
                DispatchResult::changed_with(Effect::LoadKeys {
                    backend,
                    chunk_size,
                })
            } else {
                DispatchResult::changed()
            }
        }
        Action::LoadMoreKeys(center) => {
            if app_state.is_loading_keys || !app_state.has_more_keys {
                return DispatchResult::unchanged();
            }
            app_state.is_loading_keys = true;
            if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
                let cursor = app_state.keys_cursor.clone();
                let chunk_size = app_state.keys_per_chunk;
                DispatchResult::changed_with(Effect::LoadMoreKeys {
                    backend,
                    cursor,
                    chunk_size,
                    center,
                })
            } else {
                DispatchResult::changed()
            }
        }
        Action::SelectKey(idx) => handle_select_key(app_state, ui_state, idx),
        Action::LoadValueDebounced { index, .. } => {
            // This action is now obsolete - TaskManager handles debouncing
            // Keeping for backwards compatibility during migration
            if app_state.selected_key_index == Some(index) {
                if let Some(Some(key)) = app_state.keys.get(index) {
                    if let Some(backend) = app_state.connection_manager.get_active_backend_handle()
                    {
                        let key_name = key.name.clone();
                        return DispatchResult::effect(Effect::LoadValue { backend, key_name });
                    }
                }
            }
            DispatchResult::unchanged()
        }
        Action::LoadValue { index, .. } => {
            // Load value for a specific key
            if let Some(Some(key)) = app_state.keys.get(index) {
                if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
                    let key_name = key.name.clone();
                    return DispatchResult::effect(Effect::LoadValue { backend, key_name });
                }
            }
            DispatchResult::unchanged()
        }

        // =========================================================================
        // Async results
        // =========================================================================
        Action::DidConnect(id, backend, caps) => {
            handle_did_connect(app_state, ui_state, ctx.config, id, backend, caps)
        }
        Action::DidDisconnect(id) => {
            let was_active = app_state
                .connection_manager
                .get_active_id()
                .map(|active| active == id)
                .unwrap_or(false);
            app_state
                .connection_manager
                .set_status(&id, ConnectionStatus::Disconnected);
            if was_active {
                app_state.reset_pagination();
                ui_state.key_list.select(None);
                ui_state.active_panel = Panel::Keys;
            }
            DispatchResult::changed()
        }
        Action::DidFailConnect(id, error) => {
            app_state
                .connection_manager
                .set_status(&id, ConnectionStatus::Error(error.clone()));
            app_state.error_message = Some(format!("Failed to connect: {}", error));
            DispatchResult::changed()
        }
        Action::DidScanKeys {
            keys,
            cursor,
            has_more,
            total_count,
            reset,
            center,
        } => handle_did_scan_keys(
            app_state,
            ui_state,
            keys,
            cursor,
            has_more,
            total_count,
            reset,
            center,
        ),
        Action::DidFailScanKeys(error) => {
            app_state.error_message = Some(error);
            app_state.is_loading_keys = false;
            DispatchResult::changed()
        }
        Action::DidLoadValue { value, .. } => {
            // Token check removed - TaskManager handles cancellation
            app_state.selected_value = Some(value);
            DispatchResult::changed()
        }
        Action::DidFailLoadValue(error) => {
            app_state.error_message = Some(format!("Error loading value: {}", error));
            app_state.selected_value = None;
            DispatchResult::changed()
        }

        // =========================================================================
        // CmdLine
        // =========================================================================
        Action::SetCmdLineMode(mode) => handle_set_cmdline_mode(app_state, mode),
        Action::CmdLineClear => handle_cmdline_clear(app_state, ui_state),
        Action::CmdLineHide => {
            if app_state.cmdline_mode != Some(CmdLineMode::Search) {
                app_state.reset_cmdline();
            }
            DispatchResult::changed()
        }
        Action::CmdLineConfirm => {
            // Handled in main.rs due to complex command execution
            DispatchResult::changed()
        }
        Action::CmdLineAddChar(c) => handle_cmdline_add_char(app_state, c),
        Action::CmdLineDeleteChar => handle_cmdline_delete_char(app_state),
        Action::CmdLineDeleteForward => handle_cmdline_delete_forward(app_state),
        Action::CmdLineMoveLeft => {
            if app_state.is_cmdline_active() && app_state.cmdline_cursor > 0 {
                app_state.cmdline_cursor -= 1;
            }
            DispatchResult::changed()
        }
        Action::CmdLineMoveRight => {
            let char_len = app_state.cmdline_buffer.chars().count();
            if app_state.is_cmdline_active() && app_state.cmdline_cursor < char_len {
                app_state.cmdline_cursor += 1;
            }
            DispatchResult::changed()
        }
        Action::CmdLineMoveStart => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor = 0;
            }
            DispatchResult::changed()
        }
        Action::CmdLineMoveEnd => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor = app_state.cmdline_buffer.chars().count();
            }
            DispatchResult::changed()
        }
        Action::CmdLineMoveWordLeft => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor =
                    find_word_boundary_left(&app_state.cmdline_buffer, app_state.cmdline_cursor);
            }
            DispatchResult::changed()
        }
        Action::CmdLineMoveWordRight => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_cursor =
                    find_word_boundary_right(&app_state.cmdline_buffer, app_state.cmdline_cursor);
            }
            DispatchResult::changed()
        }
        Action::CmdLineDeleteWordBack => handle_cmdline_delete_word_back(app_state),
        Action::CmdLineDeleteWordForward => handle_cmdline_delete_word_forward(app_state),
        Action::CmdLineUpdateQuery(query) => {
            if app_state.is_cmdline_active() {
                app_state.cmdline_buffer = query.clone();
                app_state.cmdline_cursor = query.chars().count();
                if matches!(
                    app_state.cmdline_mode,
                    Some(CmdLineMode::Command | CmdLineMode::Goto)
                ) {
                    app_state.suggestions_index = Some(0);
                }
                if app_state.cmdline_mode == Some(CmdLineMode::Search) {
                    return DispatchResult::changed_with(Effect::TriggerSearch);
                }
            }
            DispatchResult::changed()
        }
        Action::CmdLineNextResult | Action::CmdLinePrevResult => {
            // Handled in main.rs
            DispatchResult::changed()
        }
        Action::CmdLineHistoryPrev | Action::CmdLineHistoryNext => {
            // TODO: Implement history navigation
            DispatchResult::unchanged()
        }
        Action::CmdLineSuggestionsNext => handle_suggestions_next(app_state),
        Action::CmdLineSuggestionsPrev => handle_suggestions_prev(app_state),
        Action::CmdLineSuggestionsSelect => handle_suggestions_select(app_state),
        Action::CmdLineSetCommandResult {
            command,
            output,
            status,
            backend_type,
        } => {
            app_state.command_result =
                Some(CommandResult::new(command, output, status, backend_type));
            app_state.selected_key_index = None;
            app_state.selected_value = None;
            app_state.error_message = None;
            ui_state.key_list.select(None);
            ui_state.value_viewer.reset_scroll();
            ui_state.active_panel = Panel::Value;
            if status == CommandStatus::Success {
                app_state.cmdline_buffer.clear();
                app_state.cmdline_cursor = 0;
                if matches!(
                    app_state.cmdline_mode,
                    Some(CmdLineMode::Command | CmdLineMode::Goto)
                ) {
                    app_state.suggestions_index = Some(0);
                }
            }
            DispatchResult::changed()
        }
        Action::CmdLineClearCommandResult => {
            app_state.command_result = None;
            DispatchResult::changed()
        }

        // =========================================================================
        // Search results
        // =========================================================================
        Action::DidSearchLocal {
            indices,
            match_positions,
            ..
        } => {
            // Token check removed - TaskManager handles cancellation
            if app_state.is_search_active() {
                app_state.search_results_local = indices;
                app_state.search_match_positions = match_positions;
                if !app_state.search_results_local.is_empty() {
                    app_state.search_selection_index = Some(0);
                } else {
                    app_state.search_selection_index = None;
                }
                DispatchResult::changed()
            } else {
                DispatchResult::unchanged()
            }
        }
        Action::DidSearchServer { result, .. } => {
            // Token check removed - TaskManager handles cancellation
            app_state.is_server_searching = false;
            handle_did_search_server(app_state, result.keys)
        }

        // =========================================================================
        // Error
        // =========================================================================
        Action::Error(error) => {
            app_state.error_message = Some(error);
            DispatchResult::changed()
        }
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn handle_next_item(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        let connections_len = app_state.connection_manager.get_configs().len();
        if connections_len > 0 {
            ui_state.connection_list.next(connections_len);
        }
        return DispatchResult::changed();
    }

    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        let total_count = app_state.search_results_local.len();
        if total_count > 0 {
            let current_idx = app_state.search_selection_index.unwrap_or(0);
            let new_idx = if current_idx >= total_count - 1 {
                0
            } else {
                current_idx + 1
            };
            app_state.search_selection_index = Some(new_idx);
            // SelectKey will be sent by caller
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());

    if ui_state.next_item(keys_len) && ui_state.active_panel == Panel::Keys {
        // SelectKey will be sent by caller
    }
    DispatchResult::changed()
}

fn handle_prev_item(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        let connections_len = app_state.connection_manager.get_configs().len();
        if connections_len > 0 {
            ui_state.connection_list.prev(connections_len);
        }
        return DispatchResult::changed();
    }

    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        let total_count = app_state.search_results_local.len();
        if total_count > 0 {
            let current_idx = app_state.search_selection_index.unwrap_or(0);
            let new_idx = if current_idx == 0 {
                total_count - 1
            } else {
                current_idx - 1
            };
            app_state.search_selection_index = Some(new_idx);
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());

    if ui_state.previous_item(keys_len) && ui_state.active_panel == Panel::Keys {
        // SelectKey will be sent by caller
    }
    DispatchResult::changed()
}

fn handle_page_down(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        let connections_len = app_state.connection_manager.get_configs().len();
        if connections_len > 0 {
            ui_state.connection_list.next(connections_len);
        }
        return DispatchResult::changed();
    }

    let step = app_state.viewport_height.max(1);
    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        let total_count = app_state.search_results_local.len();
        if total_count > 0 {
            let current_idx = app_state.search_selection_index.unwrap_or(0);
            let new_idx = (current_idx + step).min(total_count.saturating_sub(1));
            app_state.search_selection_index = Some(new_idx);
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());
    if keys_len == 0 {
        return DispatchResult::changed();
    }
    let current = ui_state.key_list.state.selected().unwrap_or(0);
    let new_index = (current + step).min(keys_len.saturating_sub(1));
    if new_index != current {
        ui_state.key_list.select(Some(new_index));
    }
    DispatchResult::changed()
}

fn handle_page_up(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        let connections_len = app_state.connection_manager.get_configs().len();
        if connections_len > 0 {
            ui_state.connection_list.prev(connections_len);
        }
        return DispatchResult::changed();
    }

    let step = app_state.viewport_height.max(1);
    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        let total_count = app_state.search_results_local.len();
        if total_count > 0 {
            let current_idx = app_state.search_selection_index.unwrap_or(0);
            let new_idx = current_idx.saturating_sub(step);
            app_state.search_selection_index = Some(new_idx);
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());
    if keys_len == 0 {
        return DispatchResult::changed();
    }
    let current = ui_state.key_list.state.selected().unwrap_or(0);
    let new_index = current.saturating_sub(step);
    if new_index != current {
        ui_state.key_list.select(Some(new_index));
    }
    DispatchResult::changed()
}

fn handle_home(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        ui_state.connection_list.state.select(Some(0));
        return DispatchResult::changed();
    }

    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        if !app_state.search_results_local.is_empty() {
            app_state.search_selection_index = Some(0);
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());
    if keys_len > 0 {
        ui_state.key_list.select(Some(0));
    }
    DispatchResult::changed()
}

fn handle_end(app_state: &mut AppState, ui_state: &mut UiState) -> DispatchResult<Effect> {
    if ui_state.show_connection_palette {
        let connections_len = app_state.connection_manager.get_configs().len();
        if connections_len > 0 {
            ui_state
                .connection_list
                .state
                .select(Some(connections_len.saturating_sub(1)));
        }
        return DispatchResult::changed();
    }

    if app_state.is_search_active() && ui_state.active_panel == Panel::Keys {
        let total_count = app_state.search_results_local.len();
        if total_count > 0 {
            app_state.search_selection_index = Some(total_count.saturating_sub(1));
        }
        return DispatchResult::changed();
    }

    let keys_len = app_state
        .total_key_count
        .map(|t| t as usize)
        .unwrap_or(app_state.keys.len());
    if keys_len > 0 {
        ui_state.key_list.select(Some(keys_len.saturating_sub(1)));
    }
    DispatchResult::changed()
}

fn handle_select_key(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    idx: usize,
) -> DispatchResult<Effect> {
    app_state.command_result = None;
    ui_state.key_list.select(Some(idx));
    app_state.selected_key_index = Some(idx);
    app_state.selected_value = None;
    app_state.error_message = None;
    ui_state.value_viewer.reset_scroll();

    let mut effects = Vec::new();

    // Check if we need to load more keys
    if app_state.needs_loading_around(idx) {
        if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
            app_state.is_loading_keys = true;
            let cursor = app_state.keys_cursor.clone();
            let chunk_size = app_state.keys_per_chunk;
            effects.push(Effect::LoadMoreKeys {
                backend,
                cursor,
                chunk_size,
                center: idx,
            });
        }
    }

    // Load value for selected key (debounced via TaskManager)
    if let Some(Some(key)) = app_state.keys.get(idx) {
        if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
            let key_name = key.name.clone();
            effects.push(Effect::LoadValue { backend, key_name });
        }
    }

    if effects.is_empty() {
        DispatchResult::changed()
    } else {
        DispatchResult::changed_with_many(effects)
    }
}

fn handle_did_connect(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    config: &Config,
    id: String,
    backend: Arc<RwLock<Box<dyn Backend>>>,
    caps: BackendCapabilities,
) -> DispatchResult<Effect> {
    app_state
        .connection_manager
        .register_connection(&id, backend.clone(), caps);
    app_state.error_message = None;
    if let Ok(ids) = userdata::record_recent_connection_id_with_config(&id, config) {
        ui_state.recent_connection_ids = ids;
    }
    ui_state.active_panel = Panel::Keys;

    // Emit LoadKeys effect
    let chunk_size = app_state.keys_per_chunk;
    app_state.is_loading_keys = true;
    app_state.reset_pagination();
    DispatchResult::changed_with(Effect::LoadKeys {
        backend,
        chunk_size,
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_did_scan_keys(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    keys: Vec<KeyMetadata>,
    cursor: Option<String>,
    has_more: bool,
    total_count: Option<u64>,
    reset: bool,
    center: Option<usize>,
) -> DispatchResult<Effect> {
    if reset {
        if let Some(count) = total_count {
            app_state.total_key_count = Some(count);
            app_state.keys = vec![None; count as usize];
        } else {
            app_state.total_key_count = None;
            app_state.keys = vec![None; keys.len()];
        }
    }

    app_state.keys_cursor = cursor;
    app_state.has_more_keys = has_more;
    app_state.is_loading_keys = false;

    if reset {
        // Simple fill from start
        for (i, k) in keys.into_iter().enumerate() {
            if i < app_state.keys.len() {
                app_state.keys[i] = Some(k);
            } else if app_state.total_key_count.is_none() {
                app_state.keys.push(Some(k));
            }
        }

        // Auto-select first key if none selected
        if app_state.selected_key_index.is_none()
            && !app_state.keys.is_empty()
            && app_state.keys[0].is_some()
        {
            ui_state.key_list.select(Some(0));
            app_state.selected_key_index = Some(0);
            // Return effect to load value for first key
            if let Some(Some(key)) = app_state.keys.first() {
                if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
                    let key_name = key.name.clone();
                    return DispatchResult::changed_with(Effect::LoadValue { backend, key_name });
                }
            }
        }
    } else if let Some(c) = center {
        // Smart fill around center
        let preferred = app_state.get_preferred_indices_for_filling(c, keys.len());
        let mut keys_iter = keys.into_iter();

        for idx in preferred {
            if let Some(key) = keys_iter.next() {
                if idx < app_state.keys.len() {
                    app_state.keys[idx] = Some(key);
                }
            } else {
                break;
            }
        }

        // Fill any other empty slots with remaining keys
        for key in keys_iter {
            if let Some(empty_idx) = app_state.keys.iter().position(|k| k.is_none()) {
                app_state.keys[empty_idx] = Some(key);
            }
        }
    }

    DispatchResult::changed()
}

fn handle_did_search_server(
    app_state: &mut AppState,
    keys: Vec<KeyMetadata>,
) -> DispatchResult<Effect> {
    // Build set of local key names for deduplication
    let local_key_names: std::collections::HashSet<&str> = app_state
        .keys
        .iter()
        .filter_map(|k| k.as_ref())
        .map(|k| k.name.as_str())
        .collect();

    // Find unique server keys (not already in local keys array)
    let unique_server_keys: Vec<KeyMetadata> = keys
        .into_iter()
        .filter(|k| !local_key_names.contains(k.name.as_str()))
        .collect();

    if !unique_server_keys.is_empty() {
        // Append unique server keys to the keys array and compute match positions
        let start_idx = app_state.keys.len();
        let query = &app_state.cmdline_buffer;

        for key in unique_server_keys {
            let key_idx = app_state.keys.len();

            // Compute fuzzy match positions for highlighting
            if !query.is_empty() {
                let positions = compute_match_positions(&key.name, query);
                if !positions.is_empty() {
                    app_state.search_match_positions.insert(key_idx, positions);
                }
            }

            app_state.keys.push(Some(key));
        }

        // Add indices of server keys to search_results_local
        let server_indices: Vec<usize> = (start_idx..app_state.keys.len()).collect();
        app_state.search_results_local.extend(server_indices);

        // Update selection if needed
        if app_state.search_selection_index.is_none() && !app_state.search_results_local.is_empty()
        {
            app_state.search_selection_index = Some(0);
        }
    }

    // Clear search_results_server since we've merged them
    app_state.search_results_server.clear();
    DispatchResult::changed()
}

fn handle_set_cmdline_mode(app_state: &mut AppState, mode: CmdLineMode) -> DispatchResult<Effect> {
    if mode == CmdLineMode::Search {
        app_state.command_result = None;
    }
    if app_state.cmdline_mode == Some(mode) {
        app_state.activate_cmdline();
    } else {
        app_state.start_cmdline(mode);
    }
    if app_state.cmdline_mode == Some(CmdLineMode::Search) && !app_state.cmdline_buffer.is_empty() {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_cmdline_clear(
    app_state: &mut AppState,
    ui_state: &mut UiState,
) -> DispatchResult<Effect> {
    let was_search = app_state.cmdline_mode == Some(CmdLineMode::Search);
    if was_search {
        if app_state.cmdline_buffer.is_empty() {
            app_state.reset_cmdline();
        } else {
            app_state.cmdline_buffer.clear();
            app_state.cmdline_cursor = 0;
            app_state.cmdline_active = false;
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
    DispatchResult::changed()
}

fn handle_cmdline_add_char(app_state: &mut AppState, c: char) -> DispatchResult<Effect> {
    if !app_state.is_cmdline_active() {
        return DispatchResult::unchanged();
    }

    let byte_idx = char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    app_state.cmdline_buffer.insert(byte_idx, c);
    app_state.cmdline_cursor += 1;

    if matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        app_state.suggestions_index = Some(0);
    }

    if app_state.cmdline_mode == Some(CmdLineMode::Search) {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_cmdline_delete_char(app_state: &mut AppState) -> DispatchResult<Effect> {
    if !app_state.is_cmdline_active() || app_state.cmdline_cursor == 0 {
        return DispatchResult::changed();
    }

    app_state.cmdline_cursor -= 1;
    let byte_idx = char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    app_state.cmdline_buffer.remove(byte_idx);

    if matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        app_state.suggestions_index = Some(0);
    }

    if app_state.cmdline_buffer.is_empty() {
        if app_state.cmdline_mode == Some(CmdLineMode::Search) {
            app_state.clear_search_state();
        }
        DispatchResult::changed()
    } else if app_state.cmdline_mode == Some(CmdLineMode::Search) {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_cmdline_delete_forward(app_state: &mut AppState) -> DispatchResult<Effect> {
    let char_len = app_state.cmdline_buffer.chars().count();
    if !app_state.is_cmdline_active() || app_state.cmdline_cursor >= char_len {
        return DispatchResult::changed();
    }

    let byte_idx = char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    app_state.cmdline_buffer.remove(byte_idx);

    if matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        app_state.suggestions_index = Some(0);
    }

    if app_state.cmdline_mode == Some(CmdLineMode::Search) {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_cmdline_delete_word_back(app_state: &mut AppState) -> DispatchResult<Effect> {
    if !app_state.is_cmdline_active() || app_state.cmdline_cursor == 0 {
        return DispatchResult::changed();
    }

    let new_cursor = find_word_boundary_left(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    let start_byte = char_to_byte_index(&app_state.cmdline_buffer, new_cursor);
    let end_byte = char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    app_state.cmdline_buffer.drain(start_byte..end_byte);
    app_state.cmdline_cursor = new_cursor;

    if matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        app_state.suggestions_index = Some(0);
    }

    if app_state.cmdline_mode == Some(CmdLineMode::Search) {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_cmdline_delete_word_forward(app_state: &mut AppState) -> DispatchResult<Effect> {
    let char_len = app_state.cmdline_buffer.chars().count();
    if !app_state.is_cmdline_active() || app_state.cmdline_cursor >= char_len {
        return DispatchResult::changed();
    }

    let end = find_word_boundary_right(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    let start_byte = char_to_byte_index(&app_state.cmdline_buffer, app_state.cmdline_cursor);
    let end_byte = char_to_byte_index(&app_state.cmdline_buffer, end);
    app_state.cmdline_buffer.drain(start_byte..end_byte);

    if matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        app_state.suggestions_index = Some(0);
    }

    if app_state.cmdline_mode == Some(CmdLineMode::Search) {
        DispatchResult::changed_with(Effect::TriggerSearch)
    } else {
        DispatchResult::changed()
    }
}

fn handle_suggestions_next(app_state: &mut AppState) -> DispatchResult<Effect> {
    if !matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        return DispatchResult::unchanged();
    }

    let filtered = match app_state.cmdline_mode {
        Some(CmdLineMode::Command) => command_suggestions(&app_state.cmdline_buffer),
        Some(CmdLineMode::Goto) => goto_suggestions(&app_state.cmdline_buffer, &app_state.keys),
        _ => Vec::new(),
    };

    if !filtered.is_empty() {
        let current = app_state.suggestions_index.unwrap_or(0);
        let next = if current >= filtered.len() - 1 {
            0
        } else {
            current + 1
        };
        app_state.suggestions_index = Some(next);
    }
    DispatchResult::changed()
}

fn handle_suggestions_prev(app_state: &mut AppState) -> DispatchResult<Effect> {
    if !matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        return DispatchResult::unchanged();
    }

    let filtered = match app_state.cmdline_mode {
        Some(CmdLineMode::Command) => command_suggestions(&app_state.cmdline_buffer),
        Some(CmdLineMode::Goto) => goto_suggestions(&app_state.cmdline_buffer, &app_state.keys),
        _ => Vec::new(),
    };

    if !filtered.is_empty() {
        let current = app_state.suggestions_index.unwrap_or(0);
        let prev = if current == 0 {
            filtered.len() - 1
        } else {
            current - 1
        };
        app_state.suggestions_index = Some(prev);
    }
    DispatchResult::changed()
}

fn handle_suggestions_select(app_state: &mut AppState) -> DispatchResult<Effect> {
    if !matches!(
        app_state.cmdline_mode,
        Some(CmdLineMode::Command | CmdLineMode::Goto)
    ) {
        return DispatchResult::unchanged();
    }

    let filtered = match app_state.cmdline_mode {
        Some(CmdLineMode::Command) => command_suggestions(&app_state.cmdline_buffer),
        Some(CmdLineMode::Goto) => goto_suggestions(&app_state.cmdline_buffer, &app_state.keys),
        _ => Vec::new(),
    };

    if let Some(idx) = app_state.suggestions_index {
        if let Some(cmd) = filtered.get(idx) {
            app_state.cmdline_buffer = cmd.insert_text.clone();
            app_state.cmdline_cursor = cmd.insert_text.chars().count();
            app_state.suggestions_index = Some(0);
        }
    }
    DispatchResult::changed()
}

// =============================================================================
// String utilities
// =============================================================================

fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

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

fn compute_match_positions(key_name: &str, pattern: &str) -> Vec<u32> {
    use nucleo_matcher::{
        pattern::{CaseMatching, Normalization, Pattern},
        Config, Matcher,
    };

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pat = Pattern::parse(pattern, CaseMatching::Ignore, Normalization::Smart);

    let mut buf = Vec::new();
    let haystack = nucleo_matcher::Utf32Str::new(key_name, &mut buf);

    if pat.score(haystack, &mut matcher).is_some() {
        let mut indices_buf = Vec::new();
        pat.indices(haystack, &mut matcher, &mut indices_buf);
        indices_buf
    } else {
        Vec::new()
    }
}
