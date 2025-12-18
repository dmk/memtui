use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::layout::Rect;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Once;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use memtui::action::Action;
use memtui::actions::{async_handlers, sync_handlers};
use memtui::app::{sync_event_context, AppState, EventRunner};
use memtui::cli::{parse_connection_string, Cli, LogLevel};
use memtui::clipboard;
use memtui::config::Config;
use memtui::debug::{self, DebugFreeze, DebugOverlay};
use memtui::events::{Event, EventKind};
use memtui::keybindings::{BindingContext, KeybindingsConfig};
use memtui::terminal;
use memtui::types::{ConnectionConfig, ValueType};
use memtui::ui::components::connection_list::ConnectionListProps;
use memtui::ui::components::debug as debug_ui;
use memtui::ui::components::key_list::KeyListProps;
use memtui::ui::components::value_viewer::ValueViewerProps;
use memtui::ui::components::welcome::WelcomeScreenProps;
use memtui::ui::components::Component;
use memtui::ui::{self, init_theme, Panel, UiState};
use memtui::userdata;

pub struct App {
    pub app_state: AppState,
    pub ui_state: UiState,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,
    pub config: Config,
    pub keybindings: KeybindingsConfig,
    needs_render: bool,
    /// ID of temporary connection (from CLI connection string), if any
    temp_connection_id: Option<String>,
    /// Connection name to auto-connect to on startup
    auto_connect_name: Option<String>,
    /// Debug mode enabled via --debug flag
    debug_mode_enabled: bool,
    debug_freeze: DebugFreeze,
    pub event_runner: EventRunner,
}

impl App {
    pub fn new(cli: &Cli) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Load configuration from custom path or default
        let config = if let Some(ref config_path) = cli.config {
            userdata::load_config_from_path(config_path)
        } else {
            userdata::load_config()
        };

        // Load keybindings
        let keybindings = userdata::load_keybindings();

        let mut app_state = AppState::new_with_config(&config);
        let mut ui_state = UiState::new();

        // Load saved connections
        if let Ok(connections) = userdata::load_connections() {
            app_state.connection_manager.load_configs(connections);
        }

        if let Ok(recents) = userdata::load_recent_connection_ids() {
            ui_state.recent_connection_ids = recents;
        }

        let mut temp_connection_id = None;
        let mut auto_connect_name = None;

        // Handle CLI connection string (temporary, unsaved connection)
        if let Some(ref conn_str) = cli.connection_string {
            match parse_connection_string(conn_str) {
                Ok(parsed) => {
                    let temp_config = parsed.to_config(config.connection.default_timeout);
                    temp_connection_id = Some(temp_config.id.clone());
                    ui_state.show_temp_connection_warning = true;
                    app_state
                        .connection_manager
                        .add_connection(temp_config.clone());
                    // Auto-connect to this temporary connection
                    auto_connect_name = Some(temp_config.id.clone());
                }
                Err(e) => {
                    eprintln!("Error parsing connection string: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // Handle CLI connect by name
        if let Some(ref name) = cli.connect {
            // Find connection by name
            let configs = app_state.connection_manager.get_configs();
            if let Some(cfg) = configs.iter().find(|c| c.name == *name) {
                auto_connect_name = Some(cfg.id.clone());
            } else {
                eprintln!("Connection not found: {}", name);
                eprintln!("Available connections:");
                for cfg in configs {
                    eprintln!("  - {}", cfg.name);
                }
                std::process::exit(1);
            }
        }

        let configs = app_state.connection_manager.get_configs();
        if !configs.is_empty() {
            ui_state.connection_list.state.select(Some(0));
        }

        let event_runner = EventRunner::new(tx.clone(), &config);

        Self {
            app_state,
            ui_state,
            action_tx: tx,
            action_rx: rx,
            config,
            keybindings,
            needs_render: true, // Render on first loop iteration
            temp_connection_id,
            auto_connect_name,
            debug_mode_enabled: cli.debug,
            debug_freeze: DebugFreeze::default(),
            event_runner,
        }
    }

    /// Get the auto-connect target, if any
    pub fn take_auto_connect(&mut self) -> Option<String> {
        self.auto_connect_name.take()
    }

    /// Check if a connection is temporary (from CLI, not saved)
    pub fn is_temp_connection(&self, id: &str) -> bool {
        self.temp_connection_id.as_deref() == Some(id)
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        let tick_interval = self.config.performance.tick_interval;

        info!(
            tick_interval=?tick_interval,
            "Starting app event loop with EventRunner"
        );

        let mut interval = tokio::time::interval(tick_interval);

        loop {
            // Only render when state has changed
            if self.needs_render {
                terminal.draw(|f| {
                    if self.debug_freeze.enabled {
                        debug_ui::render_debug_freeze(
                            f,
                            &mut self.app_state,
                            &mut self.ui_state,
                            &self.keybindings,
                            &mut self.debug_freeze,
                        );
                    } else {
                        ui::render(
                            f,
                            &mut self.app_state,
                            &mut self.ui_state,
                            &self.keybindings,
                        );
                    }
                })?;
                self.needs_render = false;

                // Sync event context with current UI state
                self.event_runner.update_context(|ctx| {
                    sync_event_context(ctx, &self.ui_state, &self.app_state);
                });
            }

            if self.debug_freeze.enabled {
                tokio::select! {
                    biased;
                    // Priority 1: Debug input events
                    Some(event) = self.event_runner.poll() => {
                        self.handle_debug_event(&event).await;
                    }
                    // Priority 2: Background actions (queued until unfreeze)
                    Some(action) = self.action_rx.recv() => {
                        if matches!(action, Action::ConfirmQuit) {
                            info!("Quit confirmed while frozen, cancelling event runner");
                            self.event_runner.cancel();
                            return Ok(());
                        }
                        self.debug_freeze.queued_actions.push(action);
                    }
                    // Ignore ticks while frozen
                    _ = interval.tick() => {}
                }
                continue;
            }

            // Wait for events, actions from async tasks, or tick
            let action = tokio::select! {
                biased;
                // Priority 1: Actions from async tasks (LoadKeys results, etc.)
                Some(action) = self.action_rx.recv() => action,
                // Priority 2: Input events from EventRunner
                Some(event) = self.event_runner.poll() => {
                    let actions = self.dispatch_event(&event);
                    // Process all actions from this event
                    for action in actions {
                        // Check for quit before processing
                        if matches!(action, Action::ConfirmQuit) {
                            info!("Quit confirmed from event dispatch, cancelling event runner");
                            self.event_runner.cancel();
                            return Ok(());
                        }
                        self.update(action).await;
                    }
                    // Continue loop without waiting for tick
                    self.needs_render = true;
                    continue;
                },
                // Priority 3: Tick for animations
                _ = interval.tick() => Action::Tick,
            };

            if let Action::ConfirmQuit = action {
                // Drain any pending actions from the channel to ensure clean shutdown
                let mut drained = 0;
                while let Ok(pending) = self.action_rx.try_recv() {
                    drained += 1;
                    if !matches!(pending, Action::ConfirmQuit | Action::Tick) {
                        debug!("Draining pending action during quit");
                    }
                }
                if drained > 0 {
                    debug!(drained, "Drained pending actions before quit");
                }

                info!("Quit confirmed, cancelling event runner");
                self.event_runner.cancel();
                break;
            }

            self.update(action).await;
        }
        info!("Event loop finished");
        Ok(())
    }

    async fn handle_debug_event(&mut self, event: &Event) {
        match &event.kind {
            EventKind::Resize(_, _) => {
                self.debug_freeze.pending_capture = true;
                self.needs_render = true;
            }
            EventKind::Mouse(mouse) => {
                // Ignore scroll events entirely (don't close overlays)
                if matches!(
                    mouse.kind,
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                ) {
                    return;
                }

                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    && self.debug_freeze.mouse_capture_enabled
                {
                    if let Some(snapshot) = self.debug_freeze.snapshot.as_ref() {
                        let overlay = debug::build_inspect_overlay(
                            mouse.column,
                            mouse.row,
                            snapshot,
                            &self.app_state,
                            &self.ui_state,
                        );
                        self.debug_freeze.overlay = Some(DebugOverlay::Inspect(overlay));
                    }
                    self.debug_freeze.mouse_capture_enabled = false;
                    let _ = terminal::set_mouse_capture(false);
                    self.needs_render = true;
                }
            }
            EventKind::Key(key) => {
                if key.code == KeyCode::Esc && self.debug_freeze.overlay.is_some() {
                    self.debug_freeze.overlay = None;
                    self.needs_render = true;
                    return;
                }

                let command = self.keybindings.get_command(*key, BindingContext::Debug);
                if let Some(command) = command {
                    if command.starts_with("debug.") {
                        if let Some(action) = self.command_to_action(&command) {
                            self.update(action).await;
                            return;
                        }
                    }
                }

                // Any other key dismisses the debug overlay (if open).
                if self.debug_freeze.overlay.is_some() {
                    self.debug_freeze.overlay = None;
                    self.needs_render = true;
                }
            }
            _ => {}
        }
    }

    /// Dispatch an event to the appropriate component based on focus
    /// This is the central hub for all event handling and keybinding translation
    fn dispatch_event(&mut self, event: &Event) -> Vec<Action> {
        use memtui::ui::components::Component;

        // Handle non-key events first (mouse, scroll, resize)
        match &event.kind {
            EventKind::Mouse(mouse) => {
                return self.dispatch_mouse_event(*mouse);
            }
            EventKind::Scroll { column, row, delta } => {
                return self.dispatch_scroll_event(*column, *row, *delta);
            }
            EventKind::Resize(w, h) => {
                return vec![Action::Resize(*w, *h)];
            }
            EventKind::Tick => {
                return vec![Action::Tick];
            }
            EventKind::Key(_) => {} // Handle below
        }

        // Key events - route based on focused component
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // 1. Connection Form - handles text input directly
        if self.ui_state.show_connection_form {
            return self.dispatch_connection_form_key(event);
        }

        // 2. Help screen
        if self.ui_state.show_help {
            return self.dispatch_keybinding(*key, BindingContext::Help);
        }

        // 3. Quit confirmation
        if self.ui_state.show_quit_confirmation {
            return self.dispatch_quit_confirmation_key(event);
        }

        // 4. Connection palette
        if self.ui_state.show_connection_palette {
            let props = ConnectionListProps {
                configs: self.app_state.connection_manager.get_configs(),
                active_id: self.app_state.connection_manager.get_active_id(),
                statuses: self.app_state.connection_manager.get_statuses(),
                is_active: true,
                animation: &self.ui_state.animation,
            };

            let actions = self.ui_state.connection_list.handle_event(event, props);

            if !actions.is_empty() {
                return actions;
            }

            // Fall back to keybindings
            return self.dispatch_keybinding(*key, BindingContext::ConnectionPalette);
        }

        // 5. No active connection - welcome screen
        if self.app_state.connection_manager.get_active_id().is_none() {
            let configs = self.app_state.connection_manager.get_configs();
            let recent_configs: Vec<_> = self
                .ui_state
                .recent_connection_ids
                .iter()
                .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                .collect();

            let props = WelcomeScreenProps {
                recent_configs,
                animation: &self.ui_state.animation,
                keybindings: &self.keybindings,
            };

            let actions = self.ui_state.welcome_screen.handle_event(event, props);
            if !actions.is_empty() {
                return actions;
            }
            // Fall through to keybinding lookup
            return self.dispatch_keybinding(*key, BindingContext::Welcome);
        }

        // 6. Search mode - special handling (only when actively typing in search input)
        if self.app_state.is_searching {
            return self.dispatch_search_key(event);
        }

        // 7. Main panels - key_list or value_viewer based on active panel
        match self.ui_state.active_panel {
            Panel::Keys => {
                let active_id = self.app_state.connection_manager.get_active_id();
                let capabilities =
                    active_id.and_then(|id| self.app_state.connection_manager.get_capabilities(id));
                let active_search_query =
                    if self.app_state.is_searching || !self.app_state.search_query.is_empty() {
                        Some(self.app_state.search_query.as_str())
                    } else {
                        None
                    };
                let props = KeyListProps {
                    keys: &self.app_state.keys,
                    total_count: self.app_state.total_key_count,
                    is_loading: self.app_state.is_loading_keys,
                    active_search_query,
                    is_active: true,
                    capabilities,
                    is_searching: self.app_state.is_searching,
                    search_results_local: &self.app_state.search_results_local,
                    search_results_server: &self.app_state.search_results_server,
                    is_server_searching: self.app_state.is_server_searching,
                    search_selection_index: self.app_state.search_selection_index,
                    animation: &self.ui_state.animation,
                    search_match_positions: &self.app_state.search_match_positions,
                    now: std::time::SystemTime::now(),
                };
                let actions = self.ui_state.key_list.handle_event(event, props);
                if !actions.is_empty() {
                    return actions;
                }
            }
            Panel::Value => {
                let active_id = self.app_state.connection_manager.get_active_id();
                let capabilities =
                    active_id.and_then(|id| self.app_state.connection_manager.get_capabilities(id));
                let selected_key = self
                    .app_state
                    .selected_key_index
                    .and_then(|idx| self.app_state.keys.get(idx).and_then(|k| k.as_ref()));
                let props = ValueViewerProps {
                    selected_value: self.app_state.selected_value.as_ref(),
                    selected_key_type: selected_key.map(|k| k.value_type),
                    selected_key_name: selected_key.map(|k| k.name.as_str()),
                    selected_key,
                    error_message: self.app_state.error_message.as_ref(),
                    json_formatter: &self.app_state.json_formatter,
                    text_formatter: &self.app_state.text_formatter,
                    is_active: true,
                    capabilities,
                    animation: &self.ui_state.animation,
                    now: std::time::SystemTime::now(),
                };
                let actions = self.ui_state.value_viewer.handle_event(event, props);
                if !actions.is_empty() {
                    return actions;
                }
            }
        }

        // Fall back to default keybinding lookup
        self.dispatch_keybinding(*key, BindingContext::Default)
    }

    /// Dispatch key event for connection form
    fn dispatch_connection_form_key(&mut self, event: &Event) -> Vec<Action> {
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // Handle text input directly on form fields
        self.ui_state.connection_form.handle_key_event(*key);

        // Also check keybindings for form commands (Tab, Shift+Tab, Enter, Escape)
        self.dispatch_keybinding(*key, BindingContext::ConnectionForm)
    }

    /// Dispatch key event for quit confirmation
    fn dispatch_quit_confirmation_key(&mut self, event: &Event) -> Vec<Action> {
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };
        self.dispatch_keybinding(*key, BindingContext::QuitConfirmation)
    }

    /// Dispatch key event for search mode
    fn dispatch_search_key(&mut self, event: &Event) -> Vec<Action> {
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // Handle text input
        match key.code {
            KeyCode::Backspace if self.app_state.is_searching => {
                return vec![Action::SearchDeleteChar];
            }
            KeyCode::Char(c)
                if self.app_state.is_searching
                    && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                return vec![Action::SearchAddChar(c)];
            }
            _ => {}
        }

        // Check keybindings
        self.dispatch_keybinding(*key, BindingContext::Search)
    }

    /// Dispatch mouse event
    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> Vec<Action> {
        // Skip mouse handling for modals
        if self.ui_state.show_connection_form
            || self.ui_state.show_help
            || self.ui_state.show_quit_confirmation
        {
            return vec![];
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check resize handle
                if let Some(body_area) = self.ui_state.last_body_area {
                    if self
                        .ui_state
                        .pane_split
                        .is_on_handle(body_area, mouse.column)
                        && mouse.row >= body_area.y
                        && mouse.row < body_area.y + body_area.height
                    {
                        self.ui_state.start_resize();
                        return vec![Action::Tick];
                    }
                }
                // Delegate to existing click handler
                self.handle_mouse_click(mouse.column, mouse.row);
                vec![Action::Tick]
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.ui_state.end_resize();
                vec![Action::Tick]
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.ui_state.is_resizing {
                    if let Some(body_area) = self.ui_state.last_body_area {
                        // Calculate new ratio based on mouse position
                        let relative_x = mouse.column.saturating_sub(body_area.x) as f32;
                        let new_ratio = relative_x / body_area.width as f32;
                        self.ui_state.pane_split.ratio = new_ratio
                            .clamp(self.ui_state.pane_split.min, self.ui_state.pane_split.max);
                    }
                }
                vec![Action::Tick]
            }
            _ => vec![],
        }
    }

    /// Dispatch scroll event
    fn dispatch_scroll_event(&mut self, column: u16, row: u16, delta: isize) -> Vec<Action> {
        let event = self
            .event_runner
            .event_bus_mut()
            .create_event(EventKind::Scroll { column, row, delta });

        // 1. Try Key List
        let active_id = self.app_state.connection_manager.get_active_id();
        let capabilities =
            active_id.and_then(|id| self.app_state.connection_manager.get_capabilities(id));
        let active_search_query =
            if self.app_state.is_searching || !self.app_state.search_query.is_empty() {
                Some(self.app_state.search_query.as_str())
            } else {
                None
            };
        let key_list_props = KeyListProps {
            keys: &self.app_state.keys,
            total_count: self.app_state.total_key_count,
            is_loading: self.app_state.is_loading_keys,
            active_search_query,
            is_active: self.ui_state.active_panel == Panel::Keys,
            capabilities,
            is_searching: self.app_state.is_searching,
            search_results_local: &self.app_state.search_results_local,
            search_results_server: &self.app_state.search_results_server,
            is_server_searching: self.app_state.is_server_searching,
            search_selection_index: self.app_state.search_selection_index,
            animation: &self.ui_state.animation,
            search_match_positions: &self.app_state.search_match_positions,
            now: std::time::SystemTime::now(),
        };

        let actions = self.ui_state.key_list.handle_event(&event, key_list_props);
        if !actions.is_empty() {
            // Special handling for search navigation via scroll
            if let Some(query) = self
                .app_state
                .search_query
                .as_str()
                .split_whitespace()
                .next()
            {
                if !query.is_empty() {
                    for action in &actions {
                        if let Action::SelectKey(idx) = action {
                            if let Some(pos) = self
                                .app_state
                                .search_results_local
                                .iter()
                                .position(|i| i == idx)
                            {
                                self.app_state.search_selection_index = Some(pos);
                            }
                        }
                    }
                }
            }
            return actions;
        }

        // 2. Try Value Viewer
        let active_id = self.app_state.connection_manager.get_active_id();
        let capabilities =
            active_id.and_then(|id| self.app_state.connection_manager.get_capabilities(id));
        let selected_key = self
            .app_state
            .selected_key_index
            .and_then(|idx| self.app_state.keys.get(idx).and_then(|k| k.as_ref()));
        let value_viewer_props = ValueViewerProps {
            selected_value: self.app_state.selected_value.as_ref(),
            selected_key_type: selected_key.map(|k| k.value_type),
            selected_key_name: selected_key.map(|k| k.name.as_str()),
            selected_key,
            error_message: self.app_state.error_message.as_ref(),
            json_formatter: &self.app_state.json_formatter,
            text_formatter: &self.app_state.text_formatter,
            is_active: self.ui_state.active_panel == Panel::Value,
            capabilities,
            animation: &self.ui_state.animation,
            now: std::time::SystemTime::now(),
        };

        let actions = self
            .ui_state
            .value_viewer
            .handle_event(&event, value_viewer_props);
        if !actions.is_empty() {
            return actions;
        }

        vec![]
    }

    /// Translate key to action via keybindings config
    fn dispatch_keybinding(&self, key: event::KeyEvent, context: BindingContext) -> Vec<Action> {
        if let Some(command) = self.keybindings.get_command(key, context) {
            if let Some(action) = self.command_to_action(&command) {
                return vec![action];
            }
        }
        vec![]
    }

    /// Convert a command string to an Action
    fn command_to_action(&self, command: &str) -> Option<Action> {
        match command {
            // Quit commands
            "quit.show" => Some(Action::ShowQuitConfirmation),
            "quit.confirm" => Some(Action::ConfirmQuit),
            "quit.cancel" => Some(Action::CancelQuit),

            // Debug
            "debug.toggle" => Some(Action::ToggleDebug),
            "debug.copy_frame" => Some(Action::DebugCopyFrame),
            "debug.state.toggle" => Some(Action::DebugToggleStateView),
            "debug.mouse.toggle" => Some(Action::DebugToggleMouseCapture),

            // Help
            "help.toggle" => Some(Action::ToggleHelp),
            "help.close" => Some(Action::ToggleHelp),

            // Search commands
            "search.start" => {
                if self.app_state.connection_manager.get_active_id().is_some()
                    && !self.app_state.keys.is_empty()
                {
                    Some(Action::StartSearch)
                } else {
                    None
                }
            }
            "search.clear" => Some(Action::ClearSearch),
            "search.confirm" => Some(Action::ConfirmSearch),
            "search.next_result" => Some(Action::SearchNextResult),
            "search.prev_result" => Some(Action::SearchPrevResult),

            // Navigation commands
            "navigation.next_panel" => Some(Action::NextPanel),
            "navigation.prev_panel" => Some(Action::PrevPanel),
            "navigation.next_item" => Some(Action::NextItem),
            "navigation.prev_item" => Some(Action::PrevItem),
            "navigation.enter" => Some(Action::Enter),
            "value.view_mode.cycle" => Some(Action::CycleValueViewMode),

            // Connection commands
            "connection.palette.toggle" => {
                if self.ui_state.show_connection_palette {
                    Some(Action::CloseConnectionPalette)
                } else {
                    Some(Action::OpenConnectionPalette)
                }
            }
            "connection.palette.open" => Some(Action::OpenConnectionPalette),
            "connection.palette.close" => Some(Action::CloseConnectionPalette),
            "connection.form.open" => Some(Action::OpenConnectionForm),
            "connection.form.close" => Some(Action::CloseConnectionForm),
            "connection.form.submit" => Some(Action::Enter),
            "connection.form.next_field" => Some(Action::ConnectionFormNextField),
            "connection.form.prev_field" => Some(Action::ConnectionFormPrevField),
            "connection.tab.next" => Some(Action::NextConnectionTab),
            "connection.tab.prev" => Some(Action::PrevConnectionTab),

            _ => None,
        }
    }

    async fn update(&mut self, action: Action) {
        use async_handlers::*;
        use sync_handlers::*;

        let should_render = match action {
            // Core lifecycle
            Action::Tick => true,
            Action::Quit => false,
            Action::ConfirmQuit => false,
            Action::Resize(_, _) => true,

            // Debug (requires --debug flag)
            Action::ToggleDebug => {
                if !self.debug_mode_enabled {
                    false
                } else if self.debug_freeze.enabled {
                    let queued = std::mem::take(&mut self.debug_freeze.queued_actions);
                    self.debug_freeze = DebugFreeze::default();
                    let _ = terminal::set_mouse_capture(true);
                    for action in queued {
                        let _ = self.action_tx.send(action);
                    }
                    true
                } else {
                    self.debug_freeze.enabled = true;
                    self.debug_freeze.pending_capture = true;
                    self.debug_freeze.snapshot = None;
                    self.debug_freeze.snapshot_text.clear();
                    self.debug_freeze.queued_actions.clear();
                    self.debug_freeze.message = None;
                    self.debug_freeze.overlay = None;
                    self.debug_freeze.mouse_capture_enabled = false;
                    let _ = terminal::set_mouse_capture(false);
                    true
                }
            }
            Action::DebugCopyFrame => {
                let text = self.debug_freeze.snapshot_text.clone();
                if text.is_empty() {
                    self.debug_freeze.message = Some("No frozen frame captured yet".to_string());
                    true
                } else {
                    match clipboard::osc52_copy(&text) {
                        Ok(()) => {
                            self.debug_freeze.message =
                                Some("copied to clipboard (OSC52)".to_string())
                        }
                        Err(e) => {
                            self.debug_freeze.message = Some(format!("copy failed: {e}"));
                        }
                    }
                    true
                }
            }
            Action::DebugToggleStateView => {
                let show = !matches!(self.debug_freeze.overlay, Some(DebugOverlay::State(_)));
                if show {
                    let overlay = debug::build_state_overlay(
                        &self.app_state,
                        &self.ui_state,
                        &self.debug_freeze,
                    );
                    self.debug_freeze.overlay = Some(DebugOverlay::State(overlay));
                } else {
                    self.debug_freeze.overlay = None;
                }
                true
            }
            Action::DebugToggleMouseCapture => {
                if !self.debug_freeze.enabled {
                    false
                } else {
                    self.debug_freeze.mouse_capture_enabled =
                        !self.debug_freeze.mouse_capture_enabled;
                    if self.debug_freeze.mouse_capture_enabled {
                        let _ = terminal::set_mouse_capture(true);
                        self.debug_freeze.message =
                            Some("mouse capture ON (selection OFF)".to_string());
                    } else {
                        let _ = terminal::set_mouse_capture(false);
                        self.debug_freeze.message =
                            Some("mouse capture OFF (selection ON)".to_string());
                    }
                    true
                }
            }

            // UI toggles - delegated to sync_handlers
            Action::ShowQuitConfirmation
            | Action::CancelQuit
            | Action::ToggleHelp
            | Action::OpenConnectionForm
            | Action::CloseConnectionForm
            | Action::OpenConnectionPalette
            | Action::CloseConnectionPalette => {
                handle_ui_toggle(&mut self.ui_state, &self.app_state, &action)
            }

            // Navigation - delegated to sync_handlers
            Action::NextPanel | Action::PrevPanel | Action::NextItem | Action::PrevItem => {
                handle_navigation(
                    &mut self.ui_state,
                    &mut self.app_state,
                    &self.action_tx,
                    &action,
                )
            }

            // Value viewer
            Action::CycleValueViewMode => {
                if let Some(value) = self.app_state.selected_value.as_ref() {
                    let invalid_utf8 = std::str::from_utf8(&value.data).is_err();
                    if matches!(value.value_type, ValueType::Binary) || invalid_utf8 {
                        self.ui_state.value_viewer.cycle_view_mode();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            // Connection tabs (kept in App - accesses multiple fields)
            Action::NextConnectionTab => {
                self.cycle_connection_tab(true);
                true
            }
            Action::PrevConnectionTab => {
                self.cycle_connection_tab(false);
                true
            }

            // Connection form - delegated to sync_handlers
            Action::ConnectionFormNextField | Action::ConnectionFormPrevField => {
                handle_connection_form(
                    &mut self.ui_state,
                    &mut self.app_state,
                    &self.action_tx,
                    &self.config,
                    &action,
                )
            }
            Action::SubmitConnectionForm(_) => handle_connection_form(
                &mut self.ui_state,
                &mut self.app_state,
                &self.action_tx,
                &self.config,
                &action,
            ),

            // Enter key - context dependent
            Action::Enter => {
                if self.ui_state.show_connection_form {
                    handle_connection_form(
                        &mut self.ui_state,
                        &mut self.app_state,
                        &self.action_tx,
                        &self.config,
                        &action,
                    )
                } else if self.ui_state.show_connection_palette {
                    handle_enter_connection_palette(
                        &mut self.ui_state,
                        &self.app_state,
                        &self.action_tx,
                    )
                } else {
                    true
                }
            }

            // Connection management - delegated to sync_handlers
            Action::DeleteConnection(_) => {
                handle_connection_management(&mut self.ui_state, &mut self.app_state, &action)
            }

            // Focus/Connect/Disconnect (kept in App - complex state management)
            Action::FocusConnection(id) => {
                self.focus_connection(id);
                true
            }
            Action::Connect(id) => {
                handle_connect(&mut self.app_state, &self.action_tx, id);
                true
            }
            Action::Disconnect(id) => {
                handle_disconnect(&mut self.app_state, &mut self.ui_state, &id).await;
                true
            }

            // Async results - delegated to async_handlers
            Action::DidConnect(id, backend, caps) => {
                handle_did_connect(
                    &mut self.app_state,
                    &mut self.ui_state,
                    &self.config,
                    &self.action_tx,
                    id,
                    backend,
                    caps,
                );
                true
            }
            Action::DidFailConnect(id, error) => {
                handle_did_fail_connect(&mut self.app_state, &id, error);
                true
            }

            // Key loading - delegated to async_handlers
            Action::LoadKeys => {
                handle_load_keys(&mut self.app_state, &self.action_tx);
                true
            }
            Action::LoadMoreKeys(center) => {
                handle_load_more_keys(&mut self.app_state, &self.action_tx, center)
            }

            // Key selection (kept in App - needs schedule_value_load)
            Action::SelectKey(idx) => {
                self.app_state.selected_key_index = Some(idx);
                self.app_state.selected_value = None;
                self.app_state.error_message = None;
                self.ui_state.value_viewer.reset_scroll();
                if self.app_state.needs_loading_around(idx) {
                    let _ = self.action_tx.send(Action::LoadMoreKeys(idx));
                }
                self.schedule_value_load(idx);
                true
            }
            Action::LoadValueDebounced { index, token } => {
                if self.app_state.value_request_token == token
                    && self.app_state.selected_key_index == Some(index)
                {
                    let _ = self.action_tx.send(Action::LoadValue { index, token });
                }
                false
            }
            Action::LoadValue { index, token } => {
                handle_load_value(&self.app_state, &self.action_tx, index, token);
                false
            }

            // Scan results - delegated to async_handlers
            Action::DidScanKeys {
                keys,
                cursor,
                has_more,
                total_count,
                reset,
                center,
            } => {
                handle_did_scan_keys(
                    &mut self.app_state,
                    &mut self.ui_state,
                    &self.action_tx,
                    keys,
                    cursor,
                    has_more,
                    total_count,
                    reset,
                    center,
                );
                true
            }
            Action::DidFailScanKeys(e) => {
                handle_did_fail_scan_keys(&mut self.app_state, e);
                true
            }

            // Value results - delegated to async_handlers
            Action::DidLoadValue { value, token } => {
                handle_did_load_value(&mut self.app_state, value, token)
            }
            Action::DidFailLoadValue(e) => {
                handle_did_fail_load_value(&mut self.app_state, e);
                true
            }

            // Search - delegated to sync_handlers (with trigger_search callback)
            Action::StartSearch
            | Action::ClearSearch
            | Action::SearchAddChar(_)
            | Action::SearchDeleteChar
            | Action::UpdateSearchQuery(_) => {
                if let Some((render, needs_search)) =
                    handle_search(&mut self.app_state, &mut self.ui_state, &action)
                {
                    if needs_search {
                        trigger_search(&mut self.app_state, &self.action_tx);
                    }
                    render
                } else {
                    false
                }
            }

            // Search navigation
            Action::SearchNextResult => {
                self.handle_search_navigation(true);
                true
            }
            Action::SearchPrevResult => {
                self.handle_search_navigation(false);
                true
            }
            Action::ConfirmSearch => {
                // Exit search input mode but keep current selection
                if self.app_state.is_searching {
                    self.app_state.is_searching = false;
                }
                true
            }

            // Search results - delegated to async_handlers
            Action::DidSearchLocal {
                indices,
                match_positions,
                token,
            } => handle_did_search_local(
                &mut self.app_state,
                &self.action_tx,
                indices,
                match_positions,
                token,
            ),
            Action::DidSearchServer { result, token } => {
                handle_did_search_server(&mut self.app_state, &self.action_tx, result.keys, token)
            }

            // Error handling
            Action::Error(e) => {
                handle_error(&mut self.app_state, e);
                true
            }

            // Default
            _ => true,
        };

        if should_render {
            self.needs_render = true;
        }
    }

    /// Handle mouse click events (called from dispatch_mouse_event)
    fn handle_mouse_click(&mut self, column: u16, row: u16) {
        if self.ui_state.show_connection_palette {
            if let Some(area) = self.ui_state.connection_palette_area {
                if Self::point_in_rect(area, column, row) {
                    let total = self.app_state.connection_manager.get_configs().len();
                    if total > 0 {
                        if let Some(idx) = self
                            .ui_state
                            .connection_list
                            .index_at_position(area, column, row, total)
                        {
                            self.ui_state.connection_list.state.select(Some(idx));
                            if let Some(config) =
                                self.app_state.connection_manager.get_configs().get(idx)
                            {
                                let _ = self
                                    .action_tx
                                    .send(Action::FocusConnection(config.id.clone()));
                            }
                        }
                    }
                } else {
                    let _ = self.action_tx.send(Action::CloseConnectionPalette);
                }
            } else {
                let _ = self.action_tx.send(Action::CloseConnectionPalette);
            }
            return;
        }

        // Handle clicks on the welcome screen
        if self.app_state.connection_manager.get_active_id().is_none() {
            if let Some(area) = self.ui_state.welcome_screen.last_list_area {
                let configs = self.app_state.connection_manager.get_configs();
                let recent_configs: Vec<&ConnectionConfig> = self
                    .ui_state
                    .recent_connection_ids
                    .iter()
                    .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                    .collect();
                let total = recent_configs.len();
                if total > 0 {
                    if let Some(idx) = self
                        .ui_state
                        .welcome_screen
                        .index_at_position(area, column, row, total)
                    {
                        self.ui_state.welcome_screen.state.select(Some(idx));
                        if let Some(config) = recent_configs.get(idx) {
                            let _ = self
                                .action_tx
                                .send(Action::FocusConnection(config.id.clone()));
                        }
                    }
                }
            }
            return;
        }

        if let Some(region) = self
            .ui_state
            .tab_regions
            .iter()
            .find(|region| Self::point_in_rect(region.area, column, row))
        {
            let _ = self
                .action_tx
                .send(Action::FocusConnection(region.id.clone()));
            return;
        }

        if let Some(area) = self.ui_state.last_key_area {
            if Self::point_in_rect(area, column, row) {
                self.ui_state.active_panel = Panel::Keys;

                // Check if we're in search mode
                let has_search = !self.app_state.search_query.is_empty();

                if has_search {
                    // Search mode: click on search results
                    if let Some(result_idx) = self.search_result_index_from_position(column, row) {
                        // Update the search selection index
                        self.app_state.search_selection_index = Some(result_idx);

                        // Get the key index from search results (now includes merged server keys)
                        if let Some(&key_idx) = self.app_state.search_results_local.get(result_idx)
                        {
                            self.ui_state.key_list.select(Some(key_idx));
                            self.app_state.selected_key_index = Some(key_idx);
                            self.app_state.selected_value = None;
                            let _ = self.action_tx.send(Action::SelectKey(key_idx));
                        }
                    }
                } else {
                    // Normal mode: click on full key list
                    if let Some(index) = self.key_index_from_position(column, row) {
                        self.ui_state.key_list.select(Some(index));
                        self.app_state.selected_key_index = Some(index);
                        self.app_state.selected_value = None;
                        let _ = self.action_tx.send(Action::SelectKey(index));
                    }
                }
                return;
            }
        }

        if let Some(area) = self.ui_state.last_value_area {
            if Self::point_in_rect(area, column, row) {
                self.ui_state.active_panel = Panel::Value;
            }
        }
    }

    fn schedule_value_load(&mut self, idx: usize) {
        self.app_state.value_request_token = self.app_state.value_request_token.wrapping_add(1);
        let token = self.app_state.value_request_token;
        let tx = self.action_tx.clone();
        let debounce = self.config.data.value_load_debounce;

        tokio::spawn(async move {
            tokio::time::sleep(debounce).await;
            let _ = tx.send(Action::LoadValueDebounced { index: idx, token });
        });
    }

    /// Handle search navigation (next/prev result)
    fn handle_search_navigation(&mut self, forward: bool) {
        let total_count = self.app_state.search_results_local.len();
        if total_count == 0 {
            return;
        }

        let current_idx = self.app_state.search_selection_index.unwrap_or(0);
        let new_idx = if forward {
            if current_idx >= total_count - 1 {
                0
            } else {
                current_idx + 1
            }
        } else if current_idx == 0 {
            total_count - 1
        } else {
            current_idx - 1
        };

        self.app_state.search_selection_index = Some(new_idx);

        // Select the key using normal SelectKey action
        if let Some(&key_idx) = self.app_state.search_results_local.get(new_idx) {
            self.ui_state.key_list.select(Some(key_idx));
            self.app_state.selected_key_index = Some(key_idx);
            self.app_state.selected_value = None;
            let _ = self.action_tx.send(Action::SelectKey(key_idx));
        }
    }
    fn focus_connection(&mut self, id: String) {
        if !self.app_state.connection_manager.set_active(&id) {
            return;
        }

        self.ui_state.close_connection_palette();
        self.ui_state.active_panel = Panel::Keys;
        self.ui_state.key_list.select(None);
        self.app_state.reset_pagination();
        self.app_state.error_message = None;

        if self.app_state.connection_manager.is_connected(&id) {
            if let Ok(ids) = userdata::record_recent_connection_id_with_config(&id, &self.config) {
                self.ui_state.recent_connection_ids = ids;
            }
            let _ = self.action_tx.send(Action::LoadKeys);
        } else {
            let _ = self.action_tx.send(Action::Connect(id));
        }
    }

    fn cycle_connection_tab(&mut self, forward: bool) {
        let target_id = {
            let configs = self.app_state.connection_manager.get_configs();
            if configs.is_empty() {
                return;
            }
            let len = configs.len();
            let current_idx = self
                .app_state
                .connection_manager
                .get_active_id()
                .and_then(|active| configs.iter().position(|cfg| cfg.id == active));
            let idx = if forward {
                current_idx.map(|i| (i + 1) % len).unwrap_or(0)
            } else {
                current_idx
                    .map(|i| if i == 0 { len - 1 } else { i - 1 })
                    .unwrap_or(len.saturating_sub(1))
            };
            configs[idx].id.clone()
        };

        self.focus_connection(target_id);
    }

    fn key_index_from_position(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.ui_state.last_key_area?;
        if area.height <= 2 || area.width <= 2 {
            return None;
        }

        let inner_left = area.x.saturating_add(1);
        let inner_right = area.x.saturating_add(area.width.saturating_sub(1));
        if column < inner_left || column >= inner_right {
            return None;
        }

        let inner_top = area.y.saturating_add(1);
        let inner_bottom = area.y.saturating_add(area.height.saturating_sub(1));
        if row < inner_top || row >= inner_bottom {
            return None;
        }

        let total_count = self
            .app_state
            .total_key_count
            .map(|t| t as usize)
            .unwrap_or_else(|| self.app_state.keys.len());
        if total_count == 0 {
            return None;
        }

        let (start_index, visible_len) = self.ui_state.key_list.view_bounds(total_count)?;
        let rel = (row - inner_top) as usize;
        if rel >= visible_len {
            return None;
        }

        let index = start_index + rel;
        if index >= total_count {
            return None;
        }

        Some(index)
    }

    /// Get search result index from click position (for search mode)
    fn search_result_index_from_position(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.ui_state.last_key_area?;
        if area.height <= 2 || area.width <= 2 {
            return None;
        }

        let inner_left = area.x.saturating_add(1);
        let inner_right = area.x.saturating_add(area.width.saturating_sub(1));
        if column < inner_left || column >= inner_right {
            return None;
        }

        let inner_top = area.y.saturating_add(1);
        let inner_bottom = area.y.saturating_add(area.height.saturating_sub(1));
        if row < inner_top || row >= inner_bottom {
            return None;
        }

        let result_count = self.app_state.search_results_local.len();
        if result_count == 0 {
            return None;
        }

        let rel = (row - inner_top) as usize;
        if rel >= result_count {
            return None;
        }

        Some(rel)
    }

    fn point_in_rect(area: Rect, column: u16, row: u16) -> bool {
        let within_x = column >= area.x && column < area.x.saturating_add(area.width);
        let within_y = row >= area.y && row < area.y.saturating_add(area.height);
        within_x && within_y
    }
}

fn init_tracing(log_file: Option<&PathBuf>, log_level: LogLevel) {
    static TRACING: Once = Once::new();
    TRACING.call_once(|| {
        // Only initialize tracing if a log file is specified
        let Some(log_path) = log_file else {
            return;
        };

        // Build filter from CLI log level, but allow RUST_LOG to override
        let level_str = format!("memtui={}", log_level);
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&level_str))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // Write logs to specified file
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(log_path) {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_ansi(false)
                .with_writer(file)
                .try_init()
                .ok();
        } else {
            eprintln!("Warning: Could not open log file: {}", log_path.display());
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing (only if log file specified)
    init_tracing(cli.log_file.as_ref(), cli.log_level);

    // Load and initialize the theme before any rendering
    let theme = userdata::load_theme();
    init_theme(theme);

    // Setup panic hook to restore terminal before printing error
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = io::stdout().flush(); // Ensure buffers are flushed
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(&cli);
    info!("memtui initialized");

    // Handle auto-connect from CLI
    if let Some(id) = app.take_auto_connect() {
        let _ = app.action_tx.send(Action::FocusConnection(id));
    }

    let res = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!(?err, "Application exited with error");
        println!("Error: {:?}", err);
    } else {
        info!("Application exited cleanly");
    }

    Ok(())
}
