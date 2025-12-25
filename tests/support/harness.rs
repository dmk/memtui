use tokio::sync::mpsc;

use memtui::action::Action;
use memtui::actions::{async_handlers::trigger_search, sync_handlers};
use memtui::app::{AppState, ConnectionStatus};
use memtui::config::Config;
use memtui::keybindings::{default_keybindings, KeybindingsConfig};
use memtui::types::KeyScanResult;
use memtui::types::{ConnectionConfig, KeyMetadata};
use memtui::ui::UiState;

use super::{fixtures, render};

pub struct AppHarness {
    pub app_state: AppState,
    pub ui_state: UiState,
    #[allow(dead_code)]
    pub keybindings: KeybindingsConfig,
    #[allow(dead_code)]
    pub action_tx: mpsc::UnboundedSender<Action>,
    #[allow(dead_code)]
    pub action_rx: mpsc::UnboundedReceiver<Action>,
}

impl AppHarness {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            app_state: AppState::new_with_config(&Config::default()),
            ui_state: UiState::new(),
            keybindings: default_keybindings(),
            action_tx: tx,
            action_rx: rx,
        }
    }

    #[allow(dead_code)]
    pub fn with_connection(mut self, id: &str) -> Self {
        fixtures::attach_connected(&mut self.app_state, id);
        self.ui_state.connection_list.state.select(Some(0));
        self
    }

    #[allow(dead_code)]
    pub fn with_keys(mut self, keys: Vec<KeyMetadata>) -> Self {
        fixtures::set_keys(&mut self.app_state, keys);
        self.ui_state.key_list.select(Some(0));
        self.app_state.selected_key_index = Some(0);
        self
    }

    #[allow(dead_code)]
    pub fn with_connection_config(mut self, config: ConnectionConfig) -> Self {
        self.app_state
            .connection_manager
            .add_connection(config.clone());
        self.app_state
            .connection_manager
            .set_status(&config.id, ConnectionStatus::Connected);
        self.app_state.connection_manager.set_active(&config.id);

        // Provide default Redis-like capabilities for tests
        self.app_state.connection_manager.set_capabilities(
            &config.id,
            memtui::backend::BackendCapabilities {
                supports_ttl: true,
                supports_scan: true,
                supports_raw_commands: true,
                supports_batch_get: true,
                supports_efficient_pattern_search: true,
                supports_type_display: true,
                supports_expiry_display: true,
                has_limited_key_listing: false,
            },
        );

        self.ui_state.connection_list.state.select(Some(0));
        self
    }

    /// Render the app into a buffer for inspection.
    #[allow(dead_code)]
    pub fn render(&mut self, size: (u16, u16)) -> ratatui::buffer::Buffer {
        render::render_app(
            size,
            &mut self.app_state,
            &mut self.ui_state,
            &self.keybindings,
        )
    }

    /// Dispatch action and render, returning the buffer for assertions.
    #[allow(dead_code)]
    pub fn dispatch_and_render(
        &mut self,
        action: Action,
        size: (u16, u16),
    ) -> ratatui::buffer::Buffer {
        self.dispatch_action(action);
        self.render(size)
    }

    /// Dispatch multiple actions in sequence, returning whether any needed render.
    #[allow(dead_code)]
    pub fn dispatch_many(&mut self, actions: impl IntoIterator<Item = Action>) -> bool {
        actions
            .into_iter()
            .fold(false, |acc, a| self.dispatch_action(a) || acc)
    }

    /// Dispatch actions through the same handlers the app uses.
    /// Handles most sync actions; async result actions should use apply_* helpers.
    #[allow(dead_code)]
    pub fn dispatch_action(&mut self, action: Action) -> bool {
        match &action {
            // Search input actions
            Action::StartSearch
            | Action::ClearSearch
            | Action::SearchAddChar(_)
            | Action::SearchDeleteChar
            | Action::UpdateSearchQuery(_) => {
                if let Some((render_needed, needs_search)) =
                    sync_handlers::handle_search(&mut self.app_state, &mut self.ui_state, &action)
                {
                    if needs_search {
                        trigger_search(&mut self.app_state, &self.action_tx);
                    }
                    render_needed
                } else {
                    false
                }
            }

            // Navigation actions
            Action::NextItem | Action::PrevItem | Action::NextPanel | Action::PrevPanel => {
                sync_handlers::handle_navigation(
                    &mut self.ui_state,
                    &mut self.app_state,
                    &self.action_tx,
                    &action,
                )
            }

            // Key selection (inline - matches main.rs behavior)
            Action::SelectKey(idx) => {
                self.ui_state.key_list.select(Some(*idx));
                self.app_state.selected_key_index = Some(*idx);
                self.app_state.selected_value = None;
                self.app_state.error_message = None;
                self.ui_state.value_viewer.reset_scroll();
                true
            }

            // UI state actions (Phase 3)
            Action::FocusPanel(_)
            | Action::SetPaneRatio(_)
            | Action::SetSearchSelectionIndex(_)
            | Action::ResetKeySelection
            | Action::ExitSearchMode
            | Action::StartResize
            | Action::EndResize
            | Action::SelectConnectionIndex(_)
            | Action::SelectWelcomeIndex(_) => {
                sync_handlers::handle_ui_state(&mut self.ui_state, &mut self.app_state, &action)
            }

            // UI toggle actions
            Action::ShowQuitConfirmation
            | Action::CancelQuit
            | Action::ToggleHelp
            | Action::OpenConnectionForm
            | Action::CloseConnectionForm
            | Action::OpenConnectionPalette
            | Action::CloseConnectionPalette => {
                sync_handlers::handle_ui_toggle(&mut self.ui_state, &self.app_state, &action)
            }

            // Connection form actions
            Action::ConnectionFormNextField
            | Action::ConnectionFormPrevField
            | Action::ConnectionFormAddChar(_)
            | Action::ConnectionFormDeleteChar
            | Action::ConnectionFormDeleteForward
            | Action::ConnectionFormDeleteWordBack
            | Action::ConnectionFormDeleteWordForward
            | Action::ConnectionFormMoveLeft
            | Action::ConnectionFormMoveRight
            | Action::ConnectionFormMoveWordLeft
            | Action::ConnectionFormMoveWordRight
            | Action::ConnectionFormMoveStart
            | Action::ConnectionFormMoveEnd
            | Action::ConnectionFormNextBackendType
            | Action::ConnectionFormPrevBackendType => sync_handlers::handle_connection_form(
                &mut self.ui_state,
                &mut self.app_state,
                &self.action_tx,
                &Config::default(),
                &action,
            ),

            // Value viewer actions
            Action::ValueViewerScrollUp
            | Action::ValueViewerScrollDown
            | Action::ValueViewerScrollBy(_) => {
                sync_handlers::handle_value_viewer(&mut self.ui_state, &action)
            }

            // Connection list (palette) actions
            Action::ConnectionListNext | Action::ConnectionListPrev => {
                sync_handlers::handle_connection_list(&mut self.ui_state, &self.app_state, &action)
            }

            // Welcome screen actions
            Action::WelcomeNextItem | Action::WelcomePrevItem => {
                let configs = self.app_state.connection_manager.get_configs();
                let recent_count = self
                    .ui_state
                    .recent_connection_ids
                    .iter()
                    .filter(|id| configs.iter().any(|c| &c.id == *id))
                    .count();
                sync_handlers::handle_welcome(&mut self.ui_state, recent_count, &action)
            }

            // Connection management (delete only - connect/disconnect need async)
            Action::DeleteConnection(_) => sync_handlers::handle_connection_management(
                &mut self.ui_state,
                &mut self.app_state,
                &action,
            ),

            // Search navigation - sends sub-actions via channel
            Action::SearchNextResult | Action::SearchPrevResult | Action::ConfirmSearch => {
                // These are handled inline in main.rs but send sub-actions
                // For now, return false - use drain_actions to process emitted actions
                false
            }

            // Not handled - async actions, results, debug, etc.
            _ => false,
        }
    }

    /// Drain any queued actions from the channel (e.g. emitted by trigger_search).
    #[allow(dead_code)]
    pub fn drain_actions(&mut self) -> Vec<Action> {
        let mut out = Vec::new();
        while let Ok(action) = self.action_rx.try_recv() {
            out.push(action);
        }
        out
    }

    /// Simulate the result of a key scan without running async tasks.
    #[allow(dead_code)]
    pub fn apply_scan_result(&mut self, keys: Vec<KeyMetadata>, reset: bool, has_more: bool) {
        let total = if reset {
            Some(keys.len() as u64)
        } else {
            self.app_state.total_key_count
        };
        memtui::actions::async_handlers::handle_did_scan_keys(
            &mut self.app_state,
            &mut self.ui_state,
            &self.action_tx,
            keys,
            None,
            has_more,
            total,
            reset,
            None,
        );
    }

    /// Simulate a server search result delivery.
    #[allow(dead_code)]
    pub fn apply_server_search(&mut self, result: KeyScanResult, token: u64) {
        memtui::actions::async_handlers::handle_did_search_server(
            &mut self.app_state,
            &self.action_tx,
            result.keys,
            token,
        );
    }
}
