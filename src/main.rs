use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
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
use std::sync::{Arc, Once};
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

use memtui::action::Action;
use memtui::app::{AppState, ConnectionStatus};
use memtui::backend::{Backend, EtcdBackend, MemcachedBackend, RedisBackend};
use memtui::cli::{parse_connection_string, Cli, LogLevel};
use memtui::config::Config;
use memtui::keybindings::{BindingContext, KeybindingsConfig};
use memtui::search::fuzzy_search_keys_with_positions;
use memtui::types::{BackendType, ConnectionConfig};
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
        let event_poll_timeout = self.config.performance.event_poll_timeout;
        let event_loop_sleep = self.config.performance.event_loop_sleep;

        info!(
            tick_interval=?tick_interval,
            event_poll_timeout=?event_poll_timeout,
            event_loop_sleep=?event_loop_sleep,
            "Starting app event loop"
        );

        let mut interval = tokio::time::interval(tick_interval);

        let tx = self.action_tx.clone();

        // Create a cancellation token for clean shutdown
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let event_loop_handle = tokio::spawn(async move {
            // Maximum events to process in a single batch to prevent starvation
            const MAX_EVENTS_PER_BATCH: usize = 20;

            loop {
                tokio::select! {
                    _ = cancel_token_clone.cancelled() => {
                        info!("Event loop task cancelled");
                        // Drain any remaining events from crossterm buffer before exiting
                        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                            let _ = event::read();
                        }
                        break;
                    }
                    _ = tokio::time::sleep(event_loop_sleep) => {
                        // Process up to MAX_EVENTS_PER_BATCH events per iteration
                        // This prevents event pile-up during aggressive scrolling
                        let mut events_processed = 0;
                        while events_processed < MAX_EVENTS_PER_BATCH
                            && event::poll(event_poll_timeout).unwrap_or(false)
                        {
                            events_processed += 1;
                            if let Ok(event) = event::read() {
                                match event {
                                    Event::Key(key) => {
                                        let _ = tx.send(Action::Key(key));
                                    }
                                    Event::Mouse(mouse) => {
                                        match mouse.kind {
                                            MouseEventKind::ScrollDown => {
                                                // Send scroll events immediately for responsive scrolling
                                                let _ = tx.send(Action::Scroll {
                                                    column: mouse.column,
                                                    row: mouse.row,
                                                    delta: 1,
                                                });
                                            }
                                            MouseEventKind::ScrollUp => {
                                                // Send scroll events immediately for responsive scrolling
                                                let _ = tx.send(Action::Scroll {
                                                    column: mouse.column,
                                                    row: mouse.row,
                                                    delta: -1,
                                                });
                                            }
                                            _ => {
                                                // Forward non-scroll mouse events as normal
                                                let _ = tx.send(Action::Mouse(mouse));
                                            }
                                        }
                                    }
                                    Event::Resize(w, h) => {
                                        let _ = tx.send(Action::Resize(w, h));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        });

        loop {
            // Only render when state has changed
            if self.needs_render {
                terminal.draw(|f| {
                    ui::render(
                        f,
                        &mut self.app_state,
                        &mut self.ui_state,
                        &self.keybindings,
                    );
                })?;
                self.needs_render = false;
            }

            let action = tokio::select! {
                _ = interval.tick() => Action::Tick,
                Some(action) = self.action_rx.recv() => action,
            };

            if let Action::ConfirmQuit = action {
                // Drain any pending actions from the channel to ensure clean shutdown
                // This prevents stale events from interfering
                let mut drained = 0;
                while let Ok(pending) = self.action_rx.try_recv() {
                    drained += 1;
                    // Log non-tick/quit actions that are being drained
                    if !matches!(pending, Action::ConfirmQuit | Action::Tick) {
                        debug!("Draining pending action during quit");
                    }
                }
                if drained > 0 {
                    debug!(drained, "Drained pending actions before quit");
                }

                info!("Quit confirmed, cancelling event loop task");
                cancel_token.cancel();
                // Wait for the event loop task to finish
                if let Err(e) = event_loop_handle.await {
                    warn!("Event loop task join error: {}", e);
                }
                break;
            }

            self.update(action).await;
        }
        info!("Event loop finished");
        Ok(())
    }

    async fn update(&mut self, action: Action) {
        let should_render = match action {
            Action::Tick => true,  // Tick processes debounced scrolls and maintains FPS
            Action::Quit => false, // Legacy quit (not used with confirmation flow)
            Action::ConfirmQuit => false, // Handled in main loop
            Action::ShowQuitConfirmation => {
                self.ui_state.show_quit_confirmation = true;
                true
            }
            Action::CancelQuit => {
                self.ui_state.show_quit_confirmation = false;
                true
            }
            Action::Resize(_, _) => {
                // Resize always needs render
                true
            }
            Action::Key(key) => {
                self.handle_key(key);
                true
            }
            Action::Mouse(mouse) => {
                self.handle_mouse(mouse);
                true
            }
            Action::Scroll { column, row, delta } => {
                // Coalesce scroll events: drain pending scrolls from channel,
                // accumulating same-direction scrolls but resetting on direction change
                let mut total_delta = delta;
                let mut last_col = column;
                let mut last_row = row;

                // Drain and coalesce pending scroll events
                while let Ok(pending) = self.action_rx.try_recv() {
                    match pending {
                        Action::Scroll {
                            column: c,
                            row: r,
                            delta: d,
                        } => {
                            // Check if direction changed (sign differs)
                            let same_direction =
                                (total_delta > 0 && d > 0) || (total_delta < 0 && d < 0);
                            if same_direction {
                                // Same direction: accumulate
                                total_delta += d;
                            } else {
                                // Direction changed: reset to new direction, drop accumulated
                                total_delta = d;
                            }
                            last_col = c;
                            last_row = r;
                        }
                        other => {
                            // Non-scroll action: put it back by sending and stop draining
                            let _ = self.action_tx.send(other);
                            break;
                        }
                    }
                }

                // Process the coalesced scroll
                self.handle_scroll_delta(last_col, last_row, total_delta)
            }
            Action::NextPanel => {
                self.ui_state.next_panel();
                true
            }
            Action::PrevPanel => {
                self.ui_state.prev_panel();
                true
            }
            Action::NextConnectionTab => {
                self.cycle_connection_tab(true);
                true
            }
            Action::PrevConnectionTab => {
                self.cycle_connection_tab(false);
                true
            }

            Action::NextItem => {
                if self.ui_state.show_connection_palette {
                    let connections_len = self.app_state.connection_manager.get_configs().len();
                    if connections_len > 0 {
                        self.ui_state.connection_list.next(connections_len);
                    }
                } else {
                    let keys_len = self
                        .app_state
                        .total_key_count
                        .map(|t| t as usize)
                        .unwrap_or(self.app_state.keys.len());

                    // Pass true if the key selection actually changed
                    if self.ui_state.next_item(keys_len)
                        && self.ui_state.active_panel == Panel::Keys
                    // Only reload if we moved in the keys panel
                    {
                        if let Some(idx) = self.ui_state.key_browser.state.selected() {
                            let _ = self.action_tx.send(Action::SelectKey(idx));
                        }
                    }
                }
                true
            }
            Action::PrevItem => {
                if self.ui_state.show_connection_palette {
                    let connections_len = self.app_state.connection_manager.get_configs().len();
                    if connections_len > 0 {
                        self.ui_state.connection_list.prev(connections_len);
                    }
                } else {
                    let keys_len = self
                        .app_state
                        .total_key_count
                        .map(|t| t as usize)
                        .unwrap_or(self.app_state.keys.len());

                    // Pass true if the key selection actually changed
                    if self.ui_state.previous_item(keys_len)
                        && self.ui_state.active_panel == Panel::Keys
                    // Only reload if we moved in the keys panel
                    {
                        if let Some(idx) = self.ui_state.key_browser.state.selected() {
                            let _ = self.action_tx.send(Action::SelectKey(idx));
                        }
                    }
                }
                true
            }

            Action::OpenConnectionForm => {
                self.ui_state.open_connection_form();
                true
            }
            Action::CloseConnectionForm => {
                self.ui_state.close_connection_form();
                true
            }
            Action::ToggleHelp => {
                self.ui_state.show_help = !self.ui_state.show_help;
                true
            }
            Action::OpenConnectionPalette => {
                self.ui_state.open_connection_palette();
                let configs = self.app_state.connection_manager.get_configs();
                if configs.is_empty() {
                    self.ui_state.connection_list.state.select(None);
                } else if let Some(active_id) = self.app_state.connection_manager.get_active_id() {
                    if let Some(idx) = configs.iter().position(|cfg| cfg.id == active_id) {
                        self.ui_state.connection_list.state.select(Some(idx));
                    } else {
                        self.ui_state.connection_list.state.select(Some(0));
                    }
                } else {
                    self.ui_state.connection_list.state.select(Some(0));
                }
                true
            }
            Action::CloseConnectionPalette => {
                self.ui_state.close_connection_palette();
                true
            }

            Action::ConnectionFormNextField => {
                self.ui_state.connection_form.next_field();
                true
            }
            Action::ConnectionFormPrevField => {
                self.ui_state.connection_form.prev_field();
                true
            }

            Action::Enter => {
                if self.ui_state.show_connection_form {
                    match self
                        .ui_state
                        .connection_form
                        .to_config(self.config.connection.default_timeout)
                    {
                        Ok(config) => {
                            let _ = self.action_tx.send(Action::SubmitConnectionForm(config));
                        }
                        Err(e) => self.ui_state.set_form_error(e),
                    }
                } else if self.ui_state.show_connection_palette {
                    if let Some(idx) = self.ui_state.connection_list.state.selected() {
                        let configs = self.app_state.connection_manager.get_configs();
                        if let Some(config) = configs.get(idx) {
                            let _ = self
                                .action_tx
                                .send(Action::FocusConnection(config.id.clone()));
                        }
                    }
                    self.ui_state.close_connection_palette();
                }
                true
            }

            Action::SubmitConnectionForm(config) => {
                self.app_state
                    .connection_manager
                    .add_connection(config.clone());
                let all_configs = self.app_state.connection_manager.get_all_configs();
                let _ = userdata::save_connections(&all_configs);
                self.ui_state.close_connection_form();
                let _ = self
                    .action_tx
                    .send(Action::FocusConnection(config.id.clone()));
                true
            }

            Action::DeleteConnection(id) => {
                self.app_state.connection_manager.remove_config(&id);
                let all_configs = self.app_state.connection_manager.get_all_configs();
                let _ = userdata::save_connections(&all_configs);
                if let Ok(ids) = userdata::remove_recent_connection_id(&id) {
                    self.ui_state.recent_connection_ids = ids;
                }

                let remaining = self.app_state.connection_manager.get_configs();
                if remaining.is_empty() {
                    self.ui_state.connection_list.state.select(None);
                    self.ui_state.close_connection_palette();
                } else {
                    let current = self
                        .ui_state
                        .connection_list
                        .state
                        .selected()
                        .unwrap_or(0)
                        .min(remaining.len().saturating_sub(1));
                    self.ui_state.connection_list.state.select(Some(current));
                }
                true
            }

            Action::FocusConnection(id) => {
                self.focus_connection(id);
                true
            }

            Action::Connect(id) => {
                if let Some(config) = self.app_state.connection_manager.get_config(&id).cloned() {
                    self.app_state
                        .connection_manager
                        .set_status(&id, ConnectionStatus::Connecting);
                    let tx = self.action_tx.clone();

                    tokio::spawn(async move {
                        let mut backend: Box<dyn Backend> = match config.backend_type {
                            BackendType::Redis => Box::new(RedisBackend::new(config.clone())),
                            BackendType::Memcached => {
                                Box::new(MemcachedBackend::new(config.clone()))
                            }
                            BackendType::Etcd => Box::new(EtcdBackend::new(config.clone())),
                        };

                        match backend.connect().await {
                            Ok(_) => {
                                let backend_arc = Arc::new(RwLock::new(backend));
                                let _ = tx.send(Action::DidConnect(config.id, backend_arc));
                            }
                            Err(e) => {
                                let _ = tx.send(Action::DidFailConnect(config.id, e.to_string()));
                            }
                        }
                    });
                }
                true
            }

            Action::Disconnect(id) => {
                let was_focus = self
                    .app_state
                    .connection_manager
                    .get_active_id()
                    .map(|active| active == id)
                    .unwrap_or(false);
                let _ = self.app_state.connection_manager.disconnect(&id).await;
                if was_focus {
                    self.app_state.reset_pagination();
                    self.ui_state.key_browser.select(None);
                    self.ui_state.active_panel = Panel::Keys;
                }
                true
            }

            Action::DidConnect(id, backend) => {
                self.app_state
                    .connection_manager
                    .register_connection(&id, backend);
                self.app_state.error_message = None;
                if let Ok(ids) =
                    userdata::record_recent_connection_id_with_config(&id, &self.config)
                {
                    self.ui_state.recent_connection_ids = ids;
                }
                self.ui_state.active_panel = Panel::Keys;
                let _ = self.action_tx.send(Action::LoadKeys);
                true
            }

            Action::DidFailConnect(id, error) => {
                self.app_state
                    .connection_manager
                    .set_status(&id, ConnectionStatus::Error(error.clone()));
                self.app_state.error_message = Some(format!("Failed to connect: {}", error));
                true
            }

            Action::LoadKeys => {
                self.app_state.is_loading_keys = true;
                self.app_state.reset_pagination();

                if let Some(backend) = self
                    .app_state
                    .connection_manager
                    .get_active_backend_handle()
                {
                    let tx = self.action_tx.clone();
                    let chunk_size = self.app_state.keys_per_chunk;

                    tokio::spawn(async move {
                        let backend = backend.read().await;
                        let total = backend.key_count(None).await.ok();

                        match backend.scan_keys(None, None, chunk_size).await {
                            Ok(result) => {
                                let _ = tx.send(Action::DidScanKeys {
                                    keys: result.keys,
                                    cursor: result.cursor,
                                    has_more: result.has_more,
                                    total_count: total,
                                    reset: true,
                                    center: None,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(Action::DidFailScanKeys(e.to_string()));
                            }
                        }
                    });
                }
                true
            }

            Action::LoadMoreKeys(center) => {
                if self.app_state.is_loading_keys || !self.app_state.has_more_keys {
                    false // No state change, no render needed
                } else {
                    self.app_state.is_loading_keys = true;

                    if let Some(backend) = self
                        .app_state
                        .connection_manager
                        .get_active_backend_handle()
                    {
                        let tx = self.action_tx.clone();
                        let chunk_size = self.app_state.keys_per_chunk;
                        let cursor = self.app_state.keys_cursor.clone();

                        tokio::spawn(async move {
                            let backend = backend.read().await;
                            match backend.scan_keys(None, cursor, chunk_size).await {
                                Ok(result) => {
                                    let _ = tx.send(Action::DidScanKeys {
                                        keys: result.keys,
                                        cursor: result.cursor,
                                        has_more: result.has_more,
                                        total_count: None, // don't update total on paged load
                                        reset: false,
                                        center: Some(center),
                                    });
                                }
                                Err(e) => {
                                    let _ = tx.send(Action::DidFailScanKeys(e.to_string()));
                                }
                            }
                        });
                    }
                    true
                }
            }

            Action::SelectKey(idx) => {
                self.app_state.selected_key_index = Some(idx);
                self.app_state.selected_value = None;
                // Reset scroll for value viewer when a new key is selected
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
                false // Don't render for debounced actions, wait for actual load
            }

            Action::LoadValue { index, token } => {
                if let Some(Some(key)) = self.app_state.keys.get(index) {
                    let key_name = key.name.clone();
                    if let Some(backend) = self
                        .app_state
                        .connection_manager
                        .get_active_backend_handle()
                    {
                        let tx = self.action_tx.clone();
                        tokio::spawn(async move {
                            let backend = backend.read().await;
                            match backend.get(&key_name).await {
                                Ok(val) => {
                                    let _ = tx.send(Action::DidLoadValue { value: val, token });
                                }
                                Err(e) => {
                                    let _ = tx.send(Action::DidFailLoadValue(e.to_string()));
                                }
                            }
                        });
                    }
                } else {
                    self.app_state.selected_value = None;
                }
                false // Don't render yet, wait for DidLoadValue
            }

            Action::DidScanKeys {
                keys,
                cursor,
                has_more,
                total_count,
                reset,
                center,
            } => {
                if reset {
                    if let Some(count) = total_count {
                        self.app_state.total_key_count = Some(count);
                        self.app_state.keys = vec![None; count as usize];
                    } else {
                        // If total_count is unknown, initialize with the keys we got
                        self.app_state.total_key_count = None;
                        self.app_state.keys = vec![None; keys.len()];
                    }
                }

                self.app_state.keys_cursor = cursor;
                self.app_state.has_more_keys = has_more;
                self.app_state.is_loading_keys = false;

                if reset {
                    // Simple fill from start
                    for (i, k) in keys.into_iter().enumerate() {
                        if i < self.app_state.keys.len() {
                            self.app_state.keys[i] = Some(k);
                        } else if self.app_state.total_key_count.is_none() {
                            // If total is unknown, grow the vector as needed
                            self.app_state.keys.push(Some(k));
                        }
                    }

                    // Auto-select first key if none selected
                    if self.app_state.selected_key_index.is_none()
                        && !self.app_state.keys.is_empty()
                        && self.app_state.keys[0].is_some()
                    {
                        self.ui_state.key_browser.select(Some(0));
                        let _ = self.action_tx.send(Action::SelectKey(0));
                    }
                } else if let Some(c) = center {
                    // Smart fill around center
                    let preferred = self
                        .app_state
                        .get_preferred_indices_for_filling(c, keys.len());
                    let mut keys_iter = keys.into_iter();

                    // Fill preferred slots
                    for idx in preferred {
                        if let Some(key) = keys_iter.next() {
                            if idx < self.app_state.keys.len() {
                                self.app_state.keys[idx] = Some(key);
                            }
                        } else {
                            break;
                        }
                    }

                    // Fill any other empty slots with remaining keys
                    for key in keys_iter {
                        if let Some(empty_idx) =
                            self.app_state.keys.iter().position(|k| k.is_none())
                        {
                            self.app_state.keys[empty_idx] = Some(key);
                        }
                    }
                }
                true
            }

            Action::DidFailScanKeys(e) => {
                self.app_state.error_message = Some(e);
                self.app_state.is_loading_keys = false;
                true
            }

            Action::DidLoadValue { value, token } => {
                if self.app_state.value_request_token == token {
                    self.app_state.selected_value = Some(value);
                    true
                } else {
                    false
                }
            }

            Action::DidFailLoadValue(e) => {
                self.app_state.error_message = Some(format!("Error loading value: {}", e));
                self.app_state.selected_value = None;
                true
            }

            // Search actions
            Action::StartSearch => {
                self.app_state.start_search();
                true
            }

            Action::ClearSearch => {
                self.app_state.reset_search();
                // Reset key browser selection to show all keys
                if !self.app_state.keys.is_empty() {
                    self.ui_state.key_browser.select(Some(0));
                }
                true
            }

            Action::SearchAddChar(c) => {
                if self.app_state.is_searching {
                    self.app_state.search_query.push(c);
                    self.trigger_search();
                    true
                } else {
                    false
                }
            }

            Action::SearchDeleteChar => {
                if self.app_state.is_searching {
                    self.app_state.search_query.pop();
                    if self.app_state.search_query.is_empty() {
                        // Clear search results when query is empty
                        self.app_state.search_results_local.clear();
                        self.app_state.search_results_server.clear();
                    } else {
                        self.trigger_search();
                    }
                    true
                } else {
                    false
                }
            }

            Action::UpdateSearchQuery(query) => {
                if self.app_state.is_searching {
                    self.app_state.search_query = query;
                    self.trigger_search();
                    true
                } else {
                    false
                }
            }

            Action::DidSearchLocal {
                indices,
                match_positions,
                token,
            } => {
                if self.app_state.search_token == token && self.app_state.is_searching {
                    self.app_state.search_results_local = indices;
                    self.app_state.search_match_positions = match_positions;
                    // Auto-select first result if any
                    if !self.app_state.search_results_local.is_empty() {
                        self.app_state.search_selection_index = Some(0);
                        // Also load the value for the first result
                        if let Some(&key_idx) = self.app_state.search_results_local.first() {
                            let _ = self.action_tx.send(Action::SelectKey(key_idx));
                        }
                    } else {
                        self.app_state.search_selection_index = None;
                    }
                    true
                } else {
                    false
                }
            }

            Action::DidSearchServer { result, token } => {
                if self.app_state.search_token == token {
                    self.app_state.search_results_server = result.keys;
                    self.app_state.is_server_searching = false;
                    true
                } else {
                    false
                }
            }

            Action::Error(e) => {
                self.app_state.error_message = Some(e);
                true
            }

            _ => true, // Default: render for unknown actions (better safe than sorry)
        };

        // Set the render flag based on whether this action should trigger a render
        if should_render {
            self.needs_render = true;
        }
    }

    /// Get the current binding context based on UI state
    fn get_binding_context(&self) -> BindingContext {
        if self.ui_state.show_connection_form {
            BindingContext::ConnectionForm
        } else if self.ui_state.show_quit_confirmation {
            BindingContext::QuitConfirmation
        } else if self.ui_state.show_connection_palette {
            BindingContext::ConnectionPalette
        } else if self.app_state.is_searching || !self.app_state.search_query.is_empty() {
            BindingContext::Search
        } else if self.app_state.connection_manager.get_active_id().is_none()
            && !self.ui_state.show_connection_palette
        {
            BindingContext::Welcome
        } else {
            BindingContext::Default
        }
    }

    /// Map a command name to an Action and execute it
    fn execute_command(&mut self, command: &str) -> bool {
        match command {
            // Quit commands
            "quit.show" => {
                let _ = self.action_tx.send(Action::ShowQuitConfirmation);
                true
            }
            "quit.confirm" => {
                let _ = self.action_tx.send(Action::ConfirmQuit);
                true
            }
            "quit.cancel" => {
                let _ = self.action_tx.send(Action::CancelQuit);
                true
            }

            // Help
            "help.toggle" => {
                let _ = self.action_tx.send(Action::ToggleHelp);
                true
            }

            // Search commands
            "search.start" => {
                // Only start search when there's an active connection with keys
                if self.app_state.connection_manager.get_active_id().is_some()
                    && !self.app_state.keys.is_empty()
                {
                    let _ = self.action_tx.send(Action::StartSearch);
                }
                true
            }
            "search.clear" => {
                let _ = self.action_tx.send(Action::ClearSearch);
                true
            }
            "search.next_result" => {
                let results_len = self.app_state.search_results_local.len();
                if results_len > 0 {
                    let current = self.app_state.search_selection_index.unwrap_or(0);
                    let next = if current + 1 >= results_len {
                        0
                    } else {
                        current + 1
                    };
                    self.app_state.search_selection_index = Some(next);
                    if let Some(&key_idx) = self.app_state.search_results_local.get(next) {
                        let _ = self.action_tx.send(Action::SelectKey(key_idx));
                    }
                }
                true
            }
            "search.prev_result" => {
                let results_len = self.app_state.search_results_local.len();
                if results_len > 0 {
                    let current = self.app_state.search_selection_index.unwrap_or(0);
                    let prev = if current == 0 {
                        results_len - 1
                    } else {
                        current - 1
                    };
                    self.app_state.search_selection_index = Some(prev);
                    if let Some(&key_idx) = self.app_state.search_results_local.get(prev) {
                        let _ = self.action_tx.send(Action::SelectKey(key_idx));
                    }
                }
                true
            }
            "search.confirm" => {
                // Confirm search and keep results, exit search input mode
                if self.app_state.is_searching {
                    self.app_state.is_searching = false;
                }
                true
            }

            // Navigation commands
            "navigation.next_panel" => {
                let _ = self.action_tx.send(Action::NextPanel);
                true
            }
            "navigation.prev_panel" => {
                let _ = self.action_tx.send(Action::PrevPanel);
                true
            }
            "navigation.next_item" => {
                let context = self.get_binding_context();
                if context == BindingContext::Welcome {
                    let configs = self.app_state.connection_manager.get_configs();
                    let recent_configs: Vec<&ConnectionConfig> = self
                        .ui_state
                        .recent_connection_ids
                        .iter()
                        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                        .collect();
                    if !recent_configs.is_empty() {
                        self.ui_state.welcome_screen.next(recent_configs.len());
                    }
                } else if context == BindingContext::ConnectionPalette {
                    let configs = self.app_state.connection_manager.get_configs();
                    if !configs.is_empty() {
                        self.ui_state.connection_list.next(configs.len());
                    }
                } else {
                    let _ = self.action_tx.send(Action::NextItem);
                }
                true
            }
            "navigation.prev_item" => {
                let context = self.get_binding_context();
                if context == BindingContext::Welcome {
                    let configs = self.app_state.connection_manager.get_configs();
                    let recent_configs: Vec<&ConnectionConfig> = self
                        .ui_state
                        .recent_connection_ids
                        .iter()
                        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                        .collect();
                    if !recent_configs.is_empty() {
                        self.ui_state.welcome_screen.prev(recent_configs.len());
                    }
                } else if context == BindingContext::ConnectionPalette {
                    let configs = self.app_state.connection_manager.get_configs();
                    if !configs.is_empty() {
                        self.ui_state.connection_list.prev(configs.len());
                    }
                } else {
                    let _ = self.action_tx.send(Action::PrevItem);
                }
                true
            }
            "navigation.enter" => {
                let context = self.get_binding_context();
                if context == BindingContext::Welcome {
                    let configs = self.app_state.connection_manager.get_configs();
                    let recent_configs: Vec<&ConnectionConfig> = self
                        .ui_state
                        .recent_connection_ids
                        .iter()
                        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                        .collect();
                    if let Some(idx) = self.ui_state.welcome_screen.state.selected() {
                        if let Some(config) = recent_configs.get(idx) {
                            let _ = self
                                .action_tx
                                .send(Action::FocusConnection(config.id.clone()));
                        }
                    }
                } else if context == BindingContext::ConnectionPalette {
                    let configs = self.app_state.connection_manager.get_configs();
                    if let Some(idx) = self.ui_state.connection_list.state.selected() {
                        if let Some(config) = configs.get(idx) {
                            let _ = self
                                .action_tx
                                .send(Action::FocusConnection(config.id.clone()));
                        }
                    }
                } else {
                    let _ = self.action_tx.send(Action::Enter);
                }
                true
            }

            // Connection commands
            "connection.palette.toggle" => {
                let action = if self.ui_state.show_connection_palette {
                    Action::CloseConnectionPalette
                } else {
                    Action::OpenConnectionPalette
                };
                let _ = self.action_tx.send(action);
                true
            }
            "connection.palette.open" => {
                let _ = self.action_tx.send(Action::OpenConnectionPalette);
                true
            }
            "connection.palette.close" => {
                let _ = self.action_tx.send(Action::CloseConnectionPalette);
                true
            }
            "connection.palette.select" => {
                let configs = self.app_state.connection_manager.get_configs();
                if let Some(idx) = self.ui_state.connection_list.state.selected() {
                    if let Some(config) = configs.get(idx) {
                        let _ = self
                            .action_tx
                            .send(Action::FocusConnection(config.id.clone()));
                    }
                }
                true
            }
            "connection.palette.next" => {
                let configs = self.app_state.connection_manager.get_configs();
                if !configs.is_empty() {
                    self.ui_state.connection_list.next(configs.len());
                }
                true
            }
            "connection.palette.prev" => {
                let configs = self.app_state.connection_manager.get_configs();
                if !configs.is_empty() {
                    self.ui_state.connection_list.prev(configs.len());
                }
                true
            }
            "connection.palette.delete" => {
                let configs = self.app_state.connection_manager.get_configs();
                if let Some(idx) = self.ui_state.connection_list.state.selected() {
                    if let Some(config) = configs.get(idx) {
                        let _ = self
                            .action_tx
                            .send(Action::DeleteConnection(config.id.clone()));
                    }
                }
                true
            }
            "connection.form.open" => {
                if self.ui_state.show_connection_palette {
                    let _ = self.action_tx.send(Action::CloseConnectionPalette);
                }
                let _ = self.action_tx.send(Action::OpenConnectionForm);
                true
            }
            "connection.form.close" => {
                let _ = self.action_tx.send(Action::CloseConnectionForm);
                true
            }
            "connection.form.submit" => {
                let _ = self.action_tx.send(Action::Enter);
                true
            }
            "connection.form.next_field" => {
                let _ = self.action_tx.send(Action::ConnectionFormNextField);
                true
            }
            "connection.form.prev_field" => {
                let _ = self.action_tx.send(Action::ConnectionFormPrevField);
                true
            }
            "connection.tab.next" => {
                let _ = self.action_tx.send(Action::NextConnectionTab);
                true
            }
            "connection.tab.prev" => {
                let _ = self.action_tx.send(Action::PrevConnectionTab);
                true
            }

            // Pane resizing
            "pane.resize.left" => {
                self.ui_state.resize_panes(0.05);
                true
            }
            "pane.resize.right" => {
                self.ui_state.resize_panes(-0.05);
                true
            }

            _ => false,
        }
    }

    fn handle_key(&mut self, key: event::KeyEvent) {
        // Handle connection form input - needs direct key event handling for text input
        if self.ui_state.show_connection_form {
            self.ui_state.connection_form.handle_key_event(key);
            // Check keybindings for form-specific commands
            let context = BindingContext::ConnectionForm;
            if let Some(command) = self.keybindings.get_command(key, context) {
                let _ = self.execute_command(&command);
            }
            return;
        }

        // Help screen - any key toggles it
        if self.ui_state.show_help {
            let _ = self.action_tx.send(Action::ToggleHelp);
            return;
        }

        // Handle search mode input - special handling for character input
        if self.app_state.is_searching || !self.app_state.search_query.is_empty() {
            // Handle text input directly
            match key.code {
                KeyCode::Backspace if self.app_state.is_searching => {
                    let _ = self.action_tx.send(Action::SearchDeleteChar);
                    return;
                }
                KeyCode::Char(c)
                    if self.app_state.is_searching
                        && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    // Only handle plain character input here, let keybindings handle special keys
                    let _ = self.action_tx.send(Action::SearchAddChar(c));
                    return;
                }
                _ => {}
            }

            // Check keybindings for search commands
            let context = BindingContext::Search;
            if let Some(command) = self.keybindings.get_command(key, context) {
                let _ = self.execute_command(&command);
            }
            return;
        }

        // Determine context and look up command in keybindings
        let context = self.get_binding_context();
        if let Some(command) = self.keybindings.get_command(key, context) {
            let _ = self.execute_command(&command);
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) {
        if self.ui_state.show_connection_form
            || self.ui_state.show_help
            || self.ui_state.show_quit_confirmation
        {
            return;
        }

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if clicking on resize handle
                if let Some(body_area) = self.ui_state.last_body_area {
                    if self
                        .ui_state
                        .pane_split
                        .is_on_handle(body_area, event.column)
                        && event.row >= body_area.y
                        && event.row < body_area.y + body_area.height
                    {
                        self.ui_state.start_resize();
                        return;
                    }
                }
                self.handle_left_click(event.column, event.row);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.ui_state.end_resize();
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.ui_state.is_resizing {
                    if let Some(body_area) = self.ui_state.last_body_area {
                        // Calculate new ratio based on mouse position
                        let relative_x = event.column.saturating_sub(body_area.x) as f32;
                        let new_ratio = relative_x / body_area.width as f32;
                        let clamped = new_ratio
                            .clamp(self.ui_state.pane_split.min, self.ui_state.pane_split.max);
                        self.ui_state.pane_split.ratio = clamped;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_left_click(&mut self, column: u16, row: u16) {
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

                        // Get the actual key index from search results
                        if let Some(&key_idx) = self.app_state.search_results_local.get(result_idx)
                        {
                            self.ui_state.key_browser.select(Some(key_idx));
                            self.app_state.selected_key_index = Some(key_idx);
                            self.app_state.selected_value = None;
                            let _ = self.action_tx.send(Action::SelectKey(key_idx));
                        }
                    }
                } else {
                    // Normal mode: click on full key list
                    if let Some(index) = self.key_index_from_position(column, row) {
                        self.ui_state.key_browser.select(Some(index));
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

    /// Trigger search: runs local fuzzy search immediately and schedules background server search
    fn trigger_search(&mut self) {
        // Increment token to cancel stale searches
        self.app_state.search_token = self.app_state.search_token.wrapping_add(1);
        let token = self.app_state.search_token;
        let query = self.app_state.search_query.clone();

        if query.is_empty() {
            self.app_state.search_results_local.clear();
            self.app_state.search_results_server.clear();
            self.app_state.search_match_positions.clear();
            self.app_state.is_server_searching = false;
            return;
        }

        // 1. Immediate local fuzzy search on loaded keys
        let keys = &self.app_state.keys;
        let search_result = fuzzy_search_keys_with_positions(keys, &query);
        let tx = self.action_tx.clone();
        let _ = tx.send(Action::DidSearchLocal {
            indices: search_result.indices,
            match_positions: search_result.match_positions,
            token,
        });

        // 2. Background server search (debounced)
        if let Some(backend) = self
            .app_state
            .connection_manager
            .get_active_backend_handle()
        {
            // Mark server search as in progress
            self.app_state.is_server_searching = true;

            let tx = self.action_tx.clone();
            let search_query = query.clone();

            tokio::spawn(async move {
                // Debounce: wait 200ms before sending server request
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                let backend = backend.read().await;
                // Build pattern for server search (wrap with wildcards for substring match)
                let pattern = format!("*{}*", search_query);
                match backend.search_keys(&pattern, 100).await {
                    Ok(result) => {
                        let _ = tx.send(Action::DidSearchServer { result, token });
                    }
                    Err(e) => {
                        debug!("Server search failed: {}", e);
                        // Send empty results to clear loading state
                        let _ = tx.send(Action::DidSearchServer {
                            result: memtui::types::KeyScanResult {
                                keys: vec![],
                                cursor: None,
                                has_more: false,
                            },
                            token,
                        });
                    }
                }
            });
        }
    }

    fn focus_connection(&mut self, id: String) {
        if !self.app_state.connection_manager.set_active(&id) {
            return;
        }

        self.ui_state.close_connection_palette();
        self.ui_state.active_panel = Panel::Keys;
        self.ui_state.key_browser.select(None);
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

    #[tracing::instrument(skip(self), level = "trace")]
    fn handle_scroll_delta(&mut self, column: u16, row: u16, delta: isize) -> bool {
        if delta == 0 {
            trace!("Scroll delta is zero; ignoring");
            return false;
        }

        // Ignore scrolls when modal dialogs are open
        if self.ui_state.show_quit_confirmation
            || self.ui_state.show_help
            || self.ui_state.show_connection_form
        {
            return false;
        }

        if self.ui_state.show_connection_palette {
            if let Some(area) = self.ui_state.connection_palette_area {
                if Self::point_in_rect(area, column, row) {
                    let len = self.app_state.connection_manager.get_configs().len();
                    if len > 0 {
                        if delta > 0 {
                            // Scroll Down (next)
                            trace!(len, "Scrolling connection palette down");
                            for _ in 0..delta {
                                self.ui_state.connection_list.next(len);
                            }
                        } else {
                            // Scroll Up (prev)
                            trace!(len, "Scrolling connection palette up");
                            for _ in 0..(-delta) {
                                self.ui_state.connection_list.prev(len);
                            }
                        }
                        return true;
                    }
                }
            }
            return false;
        }

        let upward = delta < 0;
        // We want magnitude for repeated actions
        let count = delta.unsigned_abs();

        let target_panel = if let Some(area) = self.ui_state.last_key_area {
            if Self::point_in_rect(area, column, row) {
                Some(Panel::Keys)
            } else if let Some(value_area) = self.ui_state.last_value_area {
                if Self::point_in_rect(value_area, column, row) {
                    Some(Panel::Value)
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(area) = self.ui_state.last_value_area {
            if Self::point_in_rect(area, column, row) {
                Some(Panel::Value)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(panel) = target_panel {
            self.ui_state.active_panel = panel;

            if panel == Panel::Value {
                // Check if we're at a boundary before processing scroll
                let scroll_delta = if upward {
                    -(count as isize)
                } else {
                    count as isize
                };

                if let Some(at_bottom) = self.ui_state.value_viewer.is_at_boundary(scroll_delta) {
                    if at_bottom {
                        trace!("Already at bottom; dropping scroll events");
                    } else {
                        trace!("Already at top; dropping scroll events");
                    }
                    return false; // Drop scroll events when at boundary
                }

                debug!(?panel, scroll_delta, "Scrolling panel");
                self.ui_state.scroll_value_by(scroll_delta)
            } else {
                // Check if we're in search mode
                let in_search_mode = !self.app_state.search_query.is_empty();

                if in_search_mode {
                    // Scroll through search results
                    let results_len = self.app_state.search_results_local.len();
                    if results_len == 0 {
                        trace!("No search results; ignoring scroll");
                        return false;
                    }

                    let current = self.app_state.search_selection_index.unwrap_or(0);
                    let new_index = if upward {
                        // Scroll up
                        if current == 0 {
                            results_len - 1
                        } else {
                            current.saturating_sub(count)
                        }
                    } else {
                        // Scroll down
                        let next = current + count;
                        if next >= results_len {
                            0
                        } else {
                            next
                        }
                    };

                    if new_index != current {
                        self.app_state.search_selection_index = Some(new_index);
                        // Load value for selected key
                        if let Some(&key_idx) = self.app_state.search_results_local.get(new_index) {
                            let _ = self.action_tx.send(Action::SelectKey(key_idx));
                        }
                        return true;
                    }
                    false
                } else {
                    // Normal mode: scroll through all keys
                    let keys_len = self
                        .app_state
                        .total_key_count
                        .map(|t| t as usize)
                        .unwrap_or(self.app_state.keys.len());

                    let scroll_delta = if upward {
                        -(count as isize)
                    } else {
                        count as isize
                    };
                    if keys_len == 0 {
                        trace!("No keys loaded yet; ignoring key scroll");
                        return false;
                    }
                    debug!(?panel, scroll_delta, keys_len, "Scrolling panel");
                    let changed = self.ui_state.scroll_keys_by(keys_len, scroll_delta);

                    // If selection changed, trigger key selection action
                    if changed {
                        if let Some(idx) = self.ui_state.key_browser.state.selected() {
                            let _ = self.action_tx.send(Action::SelectKey(idx));
                        }
                    }

                    changed
                }
            }
        } else {
            trace!("Scroll ignored because cursor was not over a scrollable area");
            false // No state change if scroll wasn't in a valid area
        }
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

        let (start_index, visible_len) = self.ui_state.key_browser.view_bounds(total_count)?;
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
