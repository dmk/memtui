use tokio::sync::mpsc;

use memtui::action::Action;
use memtui::actions::{async_handlers::trigger_search, sync_handlers};
use memtui::app::{AppState, ConnectionStatus};
use memtui::config::Config;
use memtui::keybindings::{default_keybindings, KeybindingsConfig};
use memtui::types::KeyScanResult;
use memtui::types::{ConnectionConfig, KeyMetadata};
use memtui::ui::{Panel, UiState};

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

    /// Dispatch a subset of actions through the same handlers the app uses.
    #[allow(dead_code)]
    pub fn dispatch_action(&mut self, action: Action) -> bool {
        match &action {
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
            Action::NextItem | Action::PrevItem | Action::NextPanel | Action::PrevPanel => {
                sync_handlers::handle_navigation(
                    &mut self.ui_state,
                    &self.app_state,
                    &self.action_tx,
                    &action,
                )
            }
            Action::SelectKey(idx) => {
                self.ui_state.active_panel = Panel::Keys;
                self.ui_state.key_list.select(Some(*idx));
                self.app_state.selected_key_index = Some(*idx);
                self.app_state.selected_value = None;
                self.ui_state.value_viewer.reset_scroll();
                true
            }
            Action::OpenConnectionPalette | Action::CloseConnectionPalette => {
                sync_handlers::handle_ui_toggle(&mut self.ui_state, &self.app_state, &action)
            }
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
            result.keys,
            token,
        );
    }
}
