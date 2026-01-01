//! Centralized store with reducer pattern
//!
//! All synchronous state mutations go through the reducer.
//! Async operations are handled separately and dispatch result actions.

use tui_dispatch::debug::{ActionLogConfig, ActionLoggerMiddleware, DebugFreeze};
use tui_dispatch::{Reducer, StoreWithMiddleware};

use crate::action::Action;
use crate::app::AppState;
use crate::ui::{Panel, UiState};

/// Combined application state for the store
pub struct StoreState {
    pub app: AppState,
    pub ui: UiState,
    pub debug: DebugFreeze<Action>,
}

impl StoreState {
    pub fn new(app: AppState, ui: UiState) -> Self {
        Self {
            app,
            ui,
            debug: DebugFreeze::default(),
        }
    }
}

/// The main reducer - handles all synchronous state mutations
///
/// Returns `true` if the state changed and a re-render is needed.
pub fn reducer(state: &mut StoreState, action: Action) -> bool {
    use Action::*;

    match action {
        // ========== No-op actions (handled elsewhere) ==========
        Tick | Render | Quit | ConfirmQuit => false,

        // ========== Async intent actions (spawn tasks, no state change) ==========
        Connect(_)
        | Disconnect(_)
        | LoadKeys
        | LoadMoreKeys(_)
        | LoadValue { .. }
        | LoadValueDebounced { .. } => false,

        // ========== UI Toggles ==========
        ShowQuitConfirmation => {
            state.ui.show_quit_confirmation = true;
            true
        }
        CancelQuit => {
            state.ui.show_quit_confirmation = false;
            true
        }
        ToggleHelp => {
            state.ui.show_help = !state.ui.show_help;
            true
        }
        OpenConnectionForm => {
            state.ui.open_connection_form();
            true
        }
        CloseConnectionForm => {
            state.ui.close_connection_form();
            true
        }
        OpenConnectionPalette => {
            state.ui.open_connection_palette();
            let configs = state.app.connection_manager.get_configs();
            if configs.is_empty() {
                state.ui.connection_list.state.select(None);
            } else if let Some(active_id) = state.app.connection_manager.get_active_id() {
                if let Some(idx) = configs.iter().position(|cfg| cfg.id == active_id) {
                    state.ui.connection_list.state.select(Some(idx));
                } else {
                    state.ui.connection_list.state.select(Some(0));
                }
            } else {
                state.ui.connection_list.state.select(Some(0));
            }
            true
        }
        CloseConnectionPalette => {
            state.ui.close_connection_palette();
            true
        }

        // ========== UI State ==========
        FocusPanel(panel) => {
            if state.app.is_cmdline_active() {
                state.app.deactivate_cmdline();
            }
            state.ui.active_panel = panel;
            true
        }
        SetPaneRatio(ratio) => {
            state.ui.pane_split.ratio =
                ratio.clamp(state.ui.pane_split.min, state.ui.pane_split.max);
            true
        }
        SelectConnectionIndex(idx) => {
            state.ui.connection_list.state.select(Some(idx));
            true
        }
        SelectWelcomeIndex(idx) => {
            state.ui.welcome_screen.state.select(Some(idx));
            true
        }
        SetCmdLineSelectionIndex(idx) => {
            state.app.search_selection_index = idx;
            true
        }
        ResetKeySelection => {
            state.ui.key_list.select(None);
            state.app.reset_pagination();
            state.app.error_message = None;
            true
        }
        ExitCmdLineMode => {
            state.app.deactivate_cmdline();
            true
        }
        StartResize => {
            state.ui.start_resize();
            true
        }
        EndResize => {
            state.ui.end_resize();
            true
        }

        // ========== Value Viewer ==========
        ValueViewerScrollUp => {
            state.ui.value_viewer.scroll_up();
            true
        }
        ValueViewerScrollDown => {
            state.ui.value_viewer.scroll_down();
            true
        }
        ValueViewerScrollBy(delta) => {
            state.ui.value_viewer.scroll_by(delta as isize);
            true
        }

        // ========== Connection List (palette) ==========
        ConnectionListNext => {
            let len = state.app.connection_manager.get_configs().len();
            if len > 0 {
                state.ui.connection_list.next(len);
            }
            true
        }
        ConnectionListPrev => {
            let len = state.app.connection_manager.get_configs().len();
            if len > 0 {
                state.ui.connection_list.prev(len);
            }
            true
        }

        // ========== Welcome Screen ==========
        WelcomeNextItem => {
            let count = state.ui.recent_connection_ids.len();
            state.ui.welcome_screen.next(count);
            true
        }
        WelcomePrevItem => {
            let count = state.ui.recent_connection_ids.len();
            state.ui.welcome_screen.prev(count);
            true
        }

        // ========== Errors ==========
        Error(msg) => {
            state.app.error_message = Some(msg);
            true
        }
        DidFailScanKeys(error) => {
            state.app.error_message = Some(error);
            state.app.is_loading_keys = false;
            true
        }
        DidFailLoadValue(error) => {
            state.app.error_message = Some(format!("Error loading value: {}", error));
            state.app.selected_value = None;
            true
        }
        DidFailConnect(id, error) => {
            state.app.error_message = Some(format!("Connection {} failed: {}", id, error));
            true
        }

        // ========== Cycle Value View Mode ==========
        CycleValueViewMode => {
            if state.ui.active_panel == Panel::Value {
                state.ui.value_viewer.cycle_view_mode();
            }
            true
        }

        // ========== Connection Form (field manipulation) ==========
        ConnectionFormNextField => {
            state.ui.connection_form.next_field();
            true
        }
        ConnectionFormPrevField => {
            state.ui.connection_form.prev_field();
            true
        }
        ConnectionFormAddChar(c) => {
            state.ui.connection_form.insert_char(c);
            true
        }
        ConnectionFormDeleteChar => {
            state.ui.connection_form.delete_char();
            true
        }
        ConnectionFormDeleteForward => {
            state.ui.connection_form.delete_forward();
            true
        }
        ConnectionFormDeleteWordBack => {
            state.ui.connection_form.delete_word_back();
            true
        }
        ConnectionFormDeleteWordForward => {
            state.ui.connection_form.delete_word_forward();
            true
        }
        ConnectionFormMoveLeft => {
            state.ui.connection_form.move_left();
            true
        }
        ConnectionFormMoveRight => {
            state.ui.connection_form.move_right();
            true
        }
        ConnectionFormMoveWordLeft => {
            state.ui.connection_form.move_word_left();
            true
        }
        ConnectionFormMoveWordRight => {
            state.ui.connection_form.move_word_right();
            true
        }
        ConnectionFormMoveStart => {
            state.ui.connection_form.move_start();
            true
        }
        ConnectionFormMoveEnd => {
            state.ui.connection_form.move_end();
            true
        }
        ConnectionFormNextBackendType => {
            state.ui.connection_form.next_backend_type();
            true
        }
        ConnectionFormPrevBackendType => {
            state.ui.connection_form.prev_backend_type();
            true
        }

        // ========== Navigation (panel switching) ==========
        NextPanel => {
            if state.app.is_cmdline_active() {
                state.app.deactivate_cmdline();
            }
            state.ui.next_panel();
            true
        }
        PrevPanel => {
            if state.app.is_cmdline_active() {
                state.app.deactivate_cmdline();
            }
            state.ui.prev_panel();
            true
        }

        // TODO: Navigation item selection, CmdLine - need action_tx for dispatching follow-up actions
        // These will be migrated with ReducerResult pattern
        _ => false,
    }
}

/// Type alias for the application store with action logging middleware
pub type AppStore = StoreWithMiddleware<StoreState, Action, ActionLoggerMiddleware>;

/// Create a new store with optional action logging
pub fn create_store(app: AppState, ui: UiState, log_config: Option<ActionLogConfig>) -> AppStore {
    let state = StoreState::new(app, ui);

    let middleware = match log_config {
        Some(config) => ActionLoggerMiddleware::with_log(config),
        None => ActionLoggerMiddleware::default_filtering(),
    };

    StoreWithMiddleware::new(state, reducer as Reducer<StoreState, Action>, middleware)
}
