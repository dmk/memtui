use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::atomic::{AtomicIsize, AtomicU16, Ordering};
use std::sync::{Arc, Once};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

use memtui::action::Action;
use memtui::app::{AppState, ConnectionStatus};
use memtui::backend::{Backend, MemcachedBackend, RedisBackend};
use memtui::config::Config;
use memtui::types::{BackendType, ConnectionConfig};
use memtui::ui::{self, Panel, UiState};
use memtui::userdata;

pub struct App {
    pub app_state: AppState,
    pub ui_state: UiState,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,
    pub config: Config,
    needs_render: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Load configuration
        let config = userdata::load_config();

        let mut app_state = AppState::new_with_config(&config);
        let mut ui_state = UiState::new();

        // Load saved connections
        if let Ok(connections) = userdata::load_connections() {
            app_state.connection_manager.load_configs(connections);
        }

        if let Ok(recents) = userdata::load_recent_connection_ids() {
            ui_state.recent_connection_ids = recents;
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
            needs_render: true, // Render on first loop iteration
        }
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

        // Shared state for scroll handling
        let scroll_accumulator = Arc::new(AtomicIsize::new(0));
        let last_mouse_x = Arc::new(AtomicU16::new(0));
        let last_mouse_y = Arc::new(AtomicU16::new(0));

        let tx = self.action_tx.clone();
        let scroll_acc = scroll_accumulator.clone();
        let mouse_x = last_mouse_x.clone();
        let mouse_y = last_mouse_y.clone();

        tokio::spawn(async move {
            // Rate limit scroll events: cap maximum accumulation
            const MAX_SCROLL_ACCUMULATION: isize = 50; // ~10 lines max per batch

            loop {
                if event::poll(event_poll_timeout).unwrap_or(false)
                    && let Ok(event) = event::read()
                {
                    match event {
                        Event::Key(key) => {
                            let _ = tx.send(Action::Key(key));
                        }
                        Event::Mouse(mouse) => {
                            // Update last known mouse position (lock-free atomic operations)
                            mouse_x.store(mouse.column, Ordering::Relaxed);
                            mouse_y.store(mouse.row, Ordering::Relaxed);

                            match mouse.kind {
                                MouseEventKind::ScrollDown => {
                                    // Cap accumulation to prevent pile-up
                                    let current = scroll_acc.load(Ordering::Relaxed);
                                    if current < MAX_SCROLL_ACCUMULATION {
                                        scroll_acc.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                                MouseEventKind::ScrollUp => {
                                    // Cap accumulation to prevent pile-up
                                    let current = scroll_acc.load(Ordering::Relaxed);
                                    if current > -MAX_SCROLL_ACCUMULATION {
                                        scroll_acc.fetch_sub(1, Ordering::Relaxed);
                                    }
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
                // Yield to let other tasks run
                tokio::time::sleep(event_loop_sleep).await;
            }
        });

        loop {
            // Only render when state has changed
            if self.needs_render {
                terminal.draw(|f| {
                    ui::render(f, &mut self.app_state, &mut self.ui_state);
                })?;
                self.needs_render = false;
            }

            let action = tokio::select! {
                _ = interval.tick() => Action::Tick,
                Some(action) = self.action_rx.recv() => action,
            };

            if let Action::Quit = action {
                info!("Quit action received, breaking event loop");
                break;
            }

            // Debounced scroll processing: only on Tick
            if matches!(action, Action::Tick) {
                let scroll_delta = scroll_accumulator.swap(0, Ordering::Relaxed);
                if scroll_delta != 0 {
                    let col = last_mouse_x.load(Ordering::Relaxed);
                    let row = last_mouse_y.load(Ordering::Relaxed);
                    if self.handle_scroll_delta(col, row, scroll_delta) {
                        self.needs_render = true;
                    }
                }
            }

            self.update(action).await;
        }
        info!("Event loop finished");
        Ok(())
    }

    async fn update(&mut self, action: Action) {
        let should_render = match action {
            Action::Tick => true,  // Tick processes debounced scrolls and maintains FPS
            Action::Quit => false, // Quit is handled separately
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
                        && self.ui_state.active_panel == Panel::Keys // Only reload if we moved in the keys panel
                        && let Some(idx) = self.ui_state.key_browser.state.selected()
                    {
                        let _ = self.action_tx.send(Action::SelectKey(idx));
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
                        && self.ui_state.active_panel == Panel::Keys // Only reload if we moved in the keys panel
                        && let Some(idx) = self.ui_state.key_browser.state.selected()
                    {
                        let _ = self.action_tx.send(Action::SelectKey(idx));
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
                            BackendType::Etcd => {
                                let _ = tx.send(Action::DidFailConnect(
                                    config.id,
                                    "Etcd not implemented".into(),
                                ));
                                return;
                            }
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

    fn handle_key(&mut self, key: event::KeyEvent) {
        // Global keys
        if self.ui_state.show_connection_form {
            self.ui_state.connection_form.handle_key_event(key);
            match key.code {
                KeyCode::Enter => {
                    let _ = self.action_tx.send(Action::Enter);
                }
                KeyCode::Esc => {
                    let _ = self.action_tx.send(Action::CloseConnectionForm);
                }
                KeyCode::Tab => {
                    let _ = self.action_tx.send(Action::ConnectionFormNextField);
                }
                KeyCode::BackTab => {
                    let _ = self.action_tx.send(Action::ConnectionFormPrevField);
                }
                _ => {}
            }
            return;
        }

        if self.ui_state.show_help {
            let _ = self.action_tx.send(Action::ToggleHelp);
            return;
        }

        // Handle welcome screen navigation
        if self.app_state.connection_manager.get_active_id().is_none()
            && !self.ui_state.show_connection_palette
        {
            let configs = self.app_state.connection_manager.get_configs();
            let recent_configs: Vec<&ConnectionConfig> = self
                .ui_state
                .recent_connection_ids
                .iter()
                .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
                .collect();

            if !recent_configs.is_empty() {
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.ui_state.welcome_screen.next(recent_configs.len());
                        return;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.ui_state.welcome_screen.prev(recent_configs.len());
                        return;
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.ui_state.welcome_screen.state.selected()
                            && let Some(config) = recent_configs.get(idx)
                        {
                            let _ = self
                                .action_tx
                                .send(Action::FocusConnection(config.id.clone()));
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }

        if self.ui_state.show_connection_palette {
            let configs = self.app_state.connection_manager.get_configs();

            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('p') => {
                        let _ = self.action_tx.send(Action::CloseConnectionPalette);
                    }
                    KeyCode::Char('n') => {
                        let _ = self.action_tx.send(Action::CloseConnectionPalette);
                        let _ = self.action_tx.send(Action::OpenConnectionForm);
                    }
                    KeyCode::Tab => {
                        let _ = self.action_tx.send(Action::NextConnectionTab);
                    }
                    KeyCode::BackTab => {
                        let _ = self.action_tx.send(Action::PrevConnectionTab);
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Esc => {
                    let _ = self.action_tx.send(Action::CloseConnectionPalette);
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.ui_state.connection_list.state.selected()
                        && let Some(config) = configs.get(idx)
                    {
                        let _ = self
                            .action_tx
                            .send(Action::FocusConnection(config.id.clone()));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !configs.is_empty() {
                        self.ui_state.connection_list.next(configs.len());
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !configs.is_empty() {
                        self.ui_state.connection_list.prev(configs.len());
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(idx) = self.ui_state.connection_list.state.selected()
                        && let Some(config) = configs.get(idx)
                    {
                        let _ = self
                            .action_tx
                            .send(Action::DeleteConnection(config.id.clone()));
                    }
                }
                KeyCode::Char('q') => {
                    let _ = self.action_tx.send(Action::Quit);
                }
                KeyCode::Char('?') => {
                    let _ = self.action_tx.send(Action::ToggleHelp);
                }
                _ => {}
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') => {
                    let action = if self.ui_state.show_connection_palette {
                        Action::CloseConnectionPalette
                    } else {
                        Action::OpenConnectionPalette
                    };
                    let _ = self.action_tx.send(action);
                    return;
                }
                KeyCode::Char('n') => {
                    let _ = self.action_tx.send(Action::OpenConnectionForm);
                    return;
                }
                KeyCode::Char('f') => {
                    let _ = self.action_tx.send(Action::NextConnectionTab);
                    return;
                }
                KeyCode::Char('b') => {
                    let _ = self.action_tx.send(Action::PrevConnectionTab);
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                let _ = self.action_tx.send(Action::Quit);
            }
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::ToggleHelp);
            }
            KeyCode::Tab => {
                let _ = self.action_tx.send(Action::NextPanel);
            }
            KeyCode::BackTab => {
                let _ = self.action_tx.send(Action::PrevPanel);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let _ = self.action_tx.send(Action::NextItem);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let _ = self.action_tx.send(Action::PrevItem);
            }
            KeyCode::Enter => {
                let _ = self.action_tx.send(Action::Enter);
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(Action::Quit);
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) {
        if self.ui_state.show_connection_form || self.ui_state.show_help {
            return;
        }

        // Scroll events are now handled in handle_scroll_delta via the accumulator
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            self.handle_left_click(event.column, event.row);
        }
    }

    fn handle_left_click(&mut self, column: u16, row: u16) {
        if self.ui_state.show_connection_palette {
            if let Some(area) = self.ui_state.connection_palette_area
                && Self::point_in_rect(area, column, row)
            {
                let total = self.app_state.connection_manager.get_configs().len();
                if total > 0
                    && let Some(idx) = self
                        .ui_state
                        .connection_list
                        .index_at_position(area, column, row, total)
                {
                    self.ui_state.connection_list.state.select(Some(idx));
                    if let Some(config) = self.app_state.connection_manager.get_configs().get(idx) {
                        let _ = self
                            .action_tx
                            .send(Action::FocusConnection(config.id.clone()));
                    }
                }
            } else {
                let _ = self.action_tx.send(Action::CloseConnectionPalette);
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

        if let Some(area) = self.ui_state.last_key_area
            && Self::point_in_rect(area, column, row)
        {
            self.ui_state.active_panel = Panel::Keys;
            if let Some(index) = self.key_index_from_position(column, row) {
                self.ui_state.key_browser.select(Some(index));
                self.app_state.selected_key_index = Some(index);
                self.app_state.selected_value = None;
                let _ = self.action_tx.send(Action::SelectKey(index));
            }
            return;
        }

        if let Some(area) = self.ui_state.last_value_area
            && Self::point_in_rect(area, column, row)
        {
            self.ui_state.active_panel = Panel::Value;
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

        if self.ui_state.show_connection_palette {
            if let Some(area) = self.ui_state.connection_palette_area
                && Self::point_in_rect(area, column, row)
            {
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
            return false;
        }

        let upward = delta < 0;
        // We want magnitude for repeated actions
        let count = delta.unsigned_abs();

        let target_panel = if let Some(area) = self.ui_state.last_key_area
            && Self::point_in_rect(area, column, row)
        {
            Some(Panel::Keys)
        } else if let Some(area) = self.ui_state.last_value_area
            && Self::point_in_rect(area, column, row)
        {
            Some(Panel::Value)
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
                // Optimized batch scroll for key list (direct state update, no channel flooding)
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

    fn point_in_rect(area: Rect, column: u16, row: u16) -> bool {
        let within_x = column >= area.x && column < area.x.saturating_add(area.width);
        let within_y = row >= area.y && row < area.y.saturating_add(area.height);
        within_x && within_y
    }
}

fn init_tracing() {
    static TRACING: Once = Once::new();
    TRACING.call_once(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("memtui=info,memtui::ui=debug"))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // Write logs directly to file
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("memtui.log")
        {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_ansi(false)
                .with_writer(file)
                .try_init()
                .ok();
        } else {
            // Fallback to stderr if file can't be opened
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_ansi(false)
                .try_init()
                .ok();
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    init_tracing();

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

    let mut app = App::new();
    info!("memtui initialized");
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
