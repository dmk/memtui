use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::Duration;

use memtui::action::Action;
use memtui::app::{AppState, ConnectionStatus};
use memtui::backend::{Backend, MemcachedBackend, RedisBackend};
use memtui::types::BackendType;
use memtui::ui::{self, Panel, UiState};
use memtui::userdata;

pub struct App {
    pub app_state: AppState,
    pub ui_state: UiState,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut app_state = AppState::new();
        let mut ui_state = UiState::new();

        // Load saved connections
        if let Ok(connections) = userdata::load_connections() {
            app_state.connection_manager.load_configs(connections);
        }

        // Select first connection if any exist
        if !app_state.connection_manager.get_configs().is_empty() {
            ui_state.connection_list.state.select(Some(0));
        }

        Self {
            app_state,
            ui_state,
            action_tx: tx,
            action_rx: rx,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(250));

        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(Duration::from_millis(100)).unwrap_or(false)
                    && let Ok(event) = event::read()
                {
                    match event {
                        Event::Key(key) => {
                            let _ = tx.send(Action::Key(key));
                        }
                        Event::Mouse(mouse) => {
                            let _ = tx.send(Action::Mouse(mouse));
                        }
                        Event::Resize(w, h) => {
                            let _ = tx.send(Action::Resize(w, h));
                        }
                        _ => {}
                    }
                }
                // Yield to let other tasks run
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        loop {
            terminal.draw(|f| {
                ui::render(f, &mut self.app_state, &mut self.ui_state);
            })?;

            let action = tokio::select! {
                _ = interval.tick() => Action::Tick,
                Some(action) = self.action_rx.recv() => action,
            };

            if let Action::Quit = action {
                break;
            }

            self.update(action).await;
        }
        Ok(())
    }

    async fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {}
            Action::Quit => {}
            Action::Resize(_, _) => {}
            Action::Key(key) => {
                self.handle_key(key);
            }
            Action::Mouse(mouse) => {
                self.handle_mouse(mouse);
            }
            Action::NextPanel => self.ui_state.next_panel(),
            Action::PrevPanel => self.ui_state.prev_panel(),

            Action::NextItem => {
                let connections_len = self.app_state.connection_manager.get_configs().len();
                let keys_len = self
                    .app_state
                    .total_key_count
                    .map(|t| t as usize)
                    .unwrap_or(self.app_state.keys.len());
                if self.ui_state.next_item(connections_len, keys_len) {
                    match self.ui_state.active_panel {
                        Panel::Connections => {
                            if let Some(idx) = self.ui_state.connection_list.state.selected() {
                                let _ = self.action_tx.send(Action::SelectConnection(idx));
                            }
                        }
                        Panel::Keys => {
                            if let Some(idx) = self.ui_state.key_browser.state.selected() {
                                let _ = self.action_tx.send(Action::SelectKey(idx));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::PrevItem => {
                let connections_len = self.app_state.connection_manager.get_configs().len();
                let keys_len = self
                    .app_state
                    .total_key_count
                    .map(|t| t as usize)
                    .unwrap_or(self.app_state.keys.len());
                if self.ui_state.previous_item(connections_len, keys_len) {
                    match self.ui_state.active_panel {
                        Panel::Connections => {
                            if let Some(idx) = self.ui_state.connection_list.state.selected() {
                                let _ = self.action_tx.send(Action::SelectConnection(idx));
                            }
                        }
                        Panel::Keys => {
                            if let Some(idx) = self.ui_state.key_browser.state.selected() {
                                let _ = self.action_tx.send(Action::SelectKey(idx));
                            }
                        }
                        _ => {}
                    }
                }
            }

            Action::OpenConnectionForm => self.ui_state.open_connection_form(),
            Action::CloseConnectionForm => self.ui_state.close_connection_form(),
            Action::ToggleHelp => self.ui_state.show_help = !self.ui_state.show_help,

            Action::ConnectionFormNextField => self.ui_state.connection_form.next_field(),
            Action::ConnectionFormPrevField => self.ui_state.connection_form.prev_field(),

            Action::Enter => {
                if self.ui_state.show_connection_form {
                    match self.ui_state.connection_form.to_config() {
                        Ok(config) => {
                            let _ = self.action_tx.send(Action::SubmitConnectionForm(config));
                        }
                        Err(e) => self.ui_state.set_form_error(e),
                    }
                } else if self.ui_state.active_panel == Panel::Connections
                    && let Some(idx) = self.ui_state.connection_list.state.selected()
                    && let Some(config) = self.app_state.connection_manager.get_configs().get(idx)
                {
                    let id = config.id.clone();
                    if self.app_state.connection_manager.is_connected(&id) {
                        let _ = self.action_tx.send(Action::Disconnect(id));
                    } else {
                        let _ = self.action_tx.send(Action::Connect(id));
                    }
                } else if self.ui_state.active_panel == Panel::Connections {
                    self.ui_state.active_panel = Panel::Keys;
                }
            }

            Action::SubmitConnectionForm(config) => {
                self.app_state
                    .connection_manager
                    .add_connection(config.clone());
                let all_configs = self.app_state.connection_manager.get_all_configs();
                let _ = userdata::save_connections(&all_configs);
                let _ = self.action_tx.send(Action::Connect(config.id.clone()));
                self.ui_state.close_connection_form();
            }

            Action::DeleteConnection(id) => {
                self.app_state.connection_manager.remove_config(&id);
                let all_configs = self.app_state.connection_manager.get_all_configs();
                let _ = userdata::save_connections(&all_configs);
                self.ui_state.connection_list.state.select(None);
            }

            Action::SelectConnection(_idx) => {
                // handled by auto-update in UI for now
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
            }

            Action::Disconnect(id) => {
                let _ = self.app_state.connection_manager.disconnect(&id).await;
                if self.app_state.connection_manager.get_active_id().is_none() {
                    self.app_state.reset_pagination();
                }
            }

            Action::DidConnect(id, backend) => {
                self.app_state
                    .connection_manager
                    .register_connection(&id, backend);
                self.app_state.error_message = None;
                let _ = self.action_tx.send(Action::LoadKeys);
            }

            Action::DidFailConnect(id, error) => {
                self.app_state
                    .connection_manager
                    .set_status(&id, ConnectionStatus::Error(error.clone()));
                self.app_state.error_message = Some(format!("Failed to connect: {}", error));
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
            }

            Action::LoadMoreKeys(center) => {
                if self.app_state.is_loading_keys || !self.app_state.has_more_keys {
                    return;
                }
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
            }

            Action::SelectKey(idx) => {
                self.app_state.selected_key_index = Some(idx);
                self.app_state.selected_value = None;
                if self.app_state.needs_loading_around(idx) {
                    let _ = self.action_tx.send(Action::LoadMoreKeys(idx));
                }
                let _ = self.action_tx.send(Action::LoadValue(idx));
            }

            Action::LoadValue(idx) => {
                if let Some(Some(key)) = self.app_state.keys.get(idx) {
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
                                    let _ = tx.send(Action::DidLoadValue(val));
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
            }

            Action::DidScanKeys {
                keys,
                cursor,
                has_more,
                total_count,
                reset,
                center,
            } => {
                if reset && let Some(count) = total_count {
                    self.app_state.total_key_count = Some(count);
                    self.app_state.keys = vec![None; count as usize];
                }

                self.app_state.keys_cursor = cursor;
                self.app_state.has_more_keys = has_more;
                self.app_state.is_loading_keys = false;

                if reset {
                    // Simple fill from start
                    for (i, k) in keys.into_iter().enumerate() {
                        if i < self.app_state.keys.len() {
                            self.app_state.keys[i] = Some(k);
                        }
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
            }

            Action::DidFailScanKeys(e) => {
                self.app_state.error_message = Some(e);
                self.app_state.is_loading_keys = false;
            }

            Action::DidLoadValue(val) => {
                self.app_state.selected_value = Some(val);
            }

            Action::DidFailLoadValue(e) => {
                self.app_state.error_message = Some(format!("Error loading value: {}", e));
                self.app_state.selected_value = None;
            }

            Action::Error(e) => {
                self.app_state.error_message = Some(e);
            }

            _ => {}
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

            KeyCode::Char('n') if self.ui_state.active_panel == Panel::Connections => {
                let _ = self.action_tx.send(Action::OpenConnectionForm);
            }
            KeyCode::Char('d') if self.ui_state.active_panel == Panel::Connections => {
                if let Some(idx) = self.ui_state.connection_list.state.selected()
                    && let Some(config) = self.app_state.connection_manager.get_configs().get(idx)
                {
                    let _ = self
                        .action_tx
                        .send(Action::DeleteConnection(config.id.clone()));
                }
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) {
        if self.ui_state.show_connection_form || self.ui_state.show_help {
            return;
        }

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_left_click(event.column, event.row);
            }
            MouseEventKind::ScrollUp => {
                self.handle_scroll(event.column, event.row, true);
            }
            MouseEventKind::ScrollDown => {
                self.handle_scroll(event.column, event.row, false);
            }
            _ => {}
        }
    }

    fn handle_left_click(&mut self, column: u16, row: u16) {
        if let Some(area) = self.ui_state.last_connection_area
            && Self::point_in_rect(area, column, row)
        {
            self.ui_state.active_panel = Panel::Connections;
            let total = self.app_state.connection_manager.get_configs().len();
            if let Some(idx) = self
                .ui_state
                .connection_list
                .index_at_position(area, column, row, total)
            {
                self.ui_state.connection_list.state.select(Some(idx));
                let _ = self.action_tx.send(Action::SelectConnection(idx));
            }
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

    fn handle_scroll(&mut self, column: u16, row: u16, upward: bool) {
        let action = if upward {
            Action::PrevItem
        } else {
            Action::NextItem
        };

        if let Some(area) = self.ui_state.last_key_area
            && Self::point_in_rect(area, column, row)
        {
            self.ui_state.active_panel = Panel::Keys;
            let _ = self.action_tx.send(action);
            return;
        }

        if let Some(area) = self.ui_state.last_connection_area
            && Self::point_in_rect(area, column, row)
        {
            self.ui_state.active_panel = Panel::Connections;
            let _ = self.action_tx.send(action);
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

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
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
        println!("Error: {:?}", err);
    }

    Ok(())
}
