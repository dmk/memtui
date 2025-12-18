use super::Component;
use crate::action::Action;
use crate::events::{ComponentId, Event, EventKind, EventType};
use crate::keybindings::{BindingContext, KeybindingsConfig};
use crate::ui::widgets::InputState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::types::{Auth, BackendType, ConnectionConfig};
use crate::ui::theme::{self, raw};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    BackendType,
    Name,
    Host,
    Port,
    Password,
    Database,
}

impl FormField {
    pub fn next(&self, backend_type: BackendType) -> Self {
        match self {
            FormField::BackendType => FormField::Name,
            FormField::Name => FormField::Host,
            FormField::Host => FormField::Port,
            FormField::Port => FormField::Password,
            FormField::Password => {
                if matches!(backend_type, BackendType::Redis) {
                    FormField::Database
                } else {
                    FormField::BackendType
                }
            }
            FormField::Database => FormField::BackendType,
        }
    }

    pub fn prev(&self, backend_type: BackendType) -> Self {
        match self {
            FormField::BackendType => {
                if matches!(backend_type, BackendType::Redis) {
                    FormField::Database
                } else {
                    FormField::Password
                }
            }
            FormField::Name => FormField::BackendType,
            FormField::Host => FormField::Name,
            FormField::Port => FormField::Host,
            FormField::Password => FormField::Port,
            FormField::Database => FormField::Password,
        }
    }
}

pub struct ConnectionFormProps<'a> {
    pub keybindings: &'a KeybindingsConfig,
    pub error: Option<&'a str>,
}

pub struct ConnectionForm {
    pub name: InputState,
    pub host: InputState,
    pub port: InputState,
    pub password: InputState,
    pub database: InputState,
    pub active_field: FormField,
    pub backend_type: BackendType,
    last_area: Option<Rect>,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            name: InputState::default(),
            host: InputState::with_value("localhost"),
            port: InputState::with_value("6379"),
            password: InputState::default(),
            database: InputState::with_value("0"),
            active_field: FormField::BackendType,
            backend_type: BackendType::Redis,
            last_area: None,
        }
    }

    /// Check if the active field is a text input field
    fn is_text_field_focused(&self) -> bool {
        !matches!(self.active_field, FormField::BackendType)
    }

    /// Handle commands from keybindings
    /// Returns Some(actions) if command was handled, None if command should be ignored
    fn handle_command(&mut self, command: &str) -> Option<Vec<Action>> {
        match command {
            "connection.form.submit" => Some(vec![Action::Enter]),
            "connection.form.close" => Some(vec![Action::CloseConnectionForm]),
            "connection.form.next_field" => {
                self.next_field();
                Some(vec![Action::Tick])
            }
            "connection.form.prev_field" => {
                self.prev_field();
                Some(vec![Action::Tick])
            }
            "connection.form.backend_type.next" => {
                if self.active_field == FormField::BackendType {
                    self.next_backend_type();
                    Some(vec![Action::Tick])
                } else {
                    // Not on BackendType field, don't handle this command
                    None
                }
            }
            "connection.form.backend_type.prev" => {
                if self.active_field == FormField::BackendType {
                    self.prev_backend_type();
                    Some(vec![Action::Tick])
                } else {
                    // Not on BackendType field, don't handle this command
                    None
                }
            }
            _ => None,
        }
    }

    pub fn select_backend_type(&mut self, backend: BackendType) {
        self.backend_type = backend;
        self.port = InputState::with_value(match backend {
            BackendType::Redis => "6379",
            BackendType::Memcached => "11211",
            BackendType::Etcd => "2379",
        });
    }

    pub fn next_backend_type(&mut self) {
        let next = match self.backend_type {
            BackendType::Redis => BackendType::Memcached,
            BackendType::Memcached => BackendType::Etcd,
            BackendType::Etcd => BackendType::Redis,
        };
        self.select_backend_type(next);
    }

    pub fn prev_backend_type(&mut self) {
        let prev = match self.backend_type {
            BackendType::Redis => BackendType::Etcd,
            BackendType::Memcached => BackendType::Redis,
            BackendType::Etcd => BackendType::Memcached,
        };
        self.select_backend_type(prev);
    }

    /// Get mutable reference to the active input field
    fn active_input_mut(&mut self) -> &mut InputState {
        match self.active_field {
            FormField::Name => &mut self.name,
            FormField::Host => &mut self.host,
            FormField::Port => &mut self.port,
            FormField::Password => &mut self.password,
            FormField::Database => &mut self.database,
            FormField::BackendType => unreachable!(), // Should not be called for BackendType
        }
    }

    /// Handle text input through keybindings (only when text field is focused)
    fn handle_text_input(&mut self, key: KeyEvent, keybindings: &KeybindingsConfig) -> bool {
        // Check text input keybindings first
        if let Some(command) = keybindings.get_command(key, BindingContext::TextInput) {
            match command.as_str() {
                "input.delete_char_back" => {
                    self.active_input_mut().delete_back();
                    return true;
                }
                "input.delete_char_forward" => {
                    self.active_input_mut().delete_forward();
                    return true;
                }
                "input.delete_word_back" => {
                    self.active_input_mut().delete_word_back();
                    return true;
                }
                "input.delete_word_forward" => {
                    self.active_input_mut().delete_word_forward();
                    return true;
                }
                "input.move_word_left" => {
                    self.active_input_mut().move_word_left();
                    return true;
                }
                "input.move_word_right" => {
                    self.active_input_mut().move_word_right();
                    return true;
                }
                "input.move_start" => {
                    self.active_input_mut().move_start();
                    return true;
                }
                "input.move_end" => {
                    self.active_input_mut().move_end();
                    return true;
                }
                _ => {}
            }
        }

        // Handle arrow keys for character movement (if not bound to word movement)
        match key.code {
            KeyCode::Left if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_input_mut().move_left();
                return true;
            }
            KeyCode::Right if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_input_mut().move_right();
                return true;
            }
            _ => {}
        }

        // Printable characters without modifiers -> insert
        if let KeyCode::Char(c) = key.code {
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                self.active_input_mut().insert(c);
                return true;
            }
        }

        false
    }

    pub fn next_field(&mut self) {
        self.active_field = self.active_field.next(self.backend_type);
    }

    pub fn prev_field(&mut self) {
        self.active_field = self.active_field.prev(self.backend_type);
    }

    pub fn to_config(&self, default_timeout: Duration) -> Result<ConnectionConfig, String> {
        let name = self.name.value().trim();
        let host = self.host.value().trim();
        let port_str = self.port.value().trim();
        let password = self.password.value().trim();
        let database = self.database.value().trim();

        if name.is_empty() {
            return Err("Name is required".to_string());
        }

        if host.is_empty() {
            return Err("Host is required".to_string());
        }

        let port: u16 = port_str
            .parse()
            .map_err(|_| "Port must be a number between 1 and 65535".to_string())?;

        let auth = if !password.is_empty() {
            Some(Auth::Token(password.to_string()))
        } else {
            None
        };

        let database = match self.backend_type {
            BackendType::Redis => {
                if !database.is_empty() {
                    Some(database.to_string())
                } else {
                    Some("0".to_string())
                }
            }
            _ => None, // Memcached and Etcd don't use database field
        };

        // Generate a unique ID from name and backend type
        let id = format!(
            "{}_{}",
            name.to_lowercase().replace(' ', "_"),
            self.backend_type.to_string().to_lowercase()
        );

        Ok(ConnectionConfig {
            id,
            name: name.to_string(),
            backend_type: self.backend_type,
            host: host.to_string(),
            port,
            auth,
            database,
            tls: None,
            timeout: default_timeout,
            read_only: false,
        })
    }

    pub fn clear(&mut self) {
        self.name = InputState::default();
        self.host = InputState::with_value("localhost");
        self.backend_type = BackendType::Redis;
        self.port = InputState::with_value("6379");
        self.password = InputState::default();
        self.database = InputState::with_value("0");
        self.active_field = FormField::BackendType;
    }
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ConnectionForm {
    type Props<'a> = ConnectionFormProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        // Handle events only when focused
        if event.context.focused_component != Some(ComponentId::ConnectionForm) {
            return vec![];
        }

        if let EventKind::Key(key) = &event.kind {
            // Try to find a matching command
            if let Some(command) = props
                .keybindings
                .get_command(*key, BindingContext::ConnectionForm)
            {
                // Try to handle the command
                if let Some(actions) = self.handle_command(&command) {
                    return actions;
                }
                // Command not applicable (e.g., backend_type.next when not on BackendType)
                // Continue to check if this key should go to text input
            }

            // Text input fields get keybindings-driven input when focused
            if self.is_text_field_focused() && self.handle_text_input(*key, props.keybindings) {
                return vec![Action::Tick];
            }
        }

        vec![]
    }

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        self.last_area = Some(area);
        render_connection_form_internal(f, self, props.error, area);
    }

    fn area(&self) -> Option<Rect> {
        self.last_area
    }
}

/// Public render function (for backward compatibility)
pub fn render_connection_form(f: &mut Frame, form: &ConnectionForm, error: Option<&str>) {
    let area = theme::centered_rect(70, 70, f.area());
    render_connection_form_internal(f, form, error, area);
}

/// Internal render function (used by Component::render)
fn render_connection_form_internal(
    f: &mut Frame,
    form: &ConnectionForm,
    error: Option<&str>,
    area: Rect,
) {
    let show_database = matches!(form.backend_type, BackendType::Redis);

    let mut constraints = vec![
        Constraint::Length(3), // Title
        Constraint::Length(3), // Backend Type
        Constraint::Length(3), // Name
        Constraint::Length(3), // Host
        Constraint::Length(3), // Port
        Constraint::Length(3), // Password
    ];

    if show_database {
        constraints.push(Constraint::Length(3)); // Database (Redis only)
    }

    constraints.push(Constraint::Min(2)); // Instructions
    constraints.push(Constraint::Length(2)); // Error (if any)

    // Render modal block with raw (undimmed) colors
    let block = raw::modal_block(format!("New {} Connection", form.backend_type));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut chunk_idx = 0;

    // Title (use raw colors for modal content)
    let title = Paragraph::new(format!("Add New {} Connection", form.backend_type))
        .style(raw::form_title())
        .alignment(Alignment::Center);

    // Backend type horizontal selector
    let backends = [
        (BackendType::Redis, "Redis"),
        (BackendType::Memcached, "Memcached"),
        (BackendType::Etcd, "etcd"),
    ];

    let is_backend_active = form.active_field == FormField::BackendType;

    let backend_spans: Vec<Span> = backends
        .iter()
        .enumerate()
        .flat_map(|(i, (backend, label))| {
            let is_selected = *backend == form.backend_type;

            let style = if is_selected {
                raw::selector_option_selected(is_backend_active)
            } else {
                raw::selector_option_unselected(is_backend_active)
            };

            let mut spans = vec![Span::styled(format!(" {} ", label), style)];
            if i < backends.len() - 1 {
                spans.push(Span::raw("  "));
            }
            spans
        })
        .collect();

    let backend_type_field = Paragraph::new(Line::from(backend_spans))
        .block(raw::input_block("Backend Type (←/→)", is_backend_active));

    // Name field
    let name_field = Paragraph::new(form.name.value())
        .style(raw::input_text())
        .block(raw::input_block(
            "Name (required)",
            form.active_field == FormField::Name,
        ));

    // Host field
    let host_field = Paragraph::new(form.host.value())
        .style(raw::input_text())
        .block(raw::input_block(
            "Host",
            form.active_field == FormField::Host,
        ));

    // Port field
    let port_field = Paragraph::new(form.port.value())
        .style(raw::input_text())
        .block(raw::input_block(
            "Port",
            form.active_field == FormField::Port,
        ));

    let password_val = form.password.value();
    let password_display = if password_val.is_empty() {
        String::new()
    } else {
        "*".repeat(password_val.len())
    };
    let password_field = Paragraph::new(password_display)
        .style(raw::input_text())
        .block(raw::input_block(
            "Password (optional)",
            form.active_field == FormField::Password,
        ));

    // Database field
    let database_field = Paragraph::new(form.database.value())
        .style(raw::input_text())
        .block(raw::input_block(
            "Database",
            form.active_field == FormField::Database,
        ));

    let mut instructions = vec![Line::from("")];

    if matches!(form.backend_type, BackendType::Memcached) {
        instructions.push(Line::from(vec![Span::styled(
            "Note: Memcached doesn't use database field",
            raw::form_note(),
        )]));
    } else if matches!(form.backend_type, BackendType::Etcd) {
        instructions.push(Line::from(vec![Span::styled(
            "Note: etcd doesn't use database field",
            raw::form_note(),
        )]));
    }

    instructions.push(Line::from(vec![
        Span::styled("Tab/↓", raw::keybind_key_style()),
        Span::styled(" Next  ", raw::form_hint()),
        Span::styled("Shift+Tab/↑", raw::keybind_key_style()),
        Span::styled(" Prev  ", raw::form_hint()),
        Span::styled("Enter", raw::keybind_key_style()),
        Span::styled(" Save  ", raw::form_hint()),
        Span::styled("Esc", raw::keybind_key_style()),
        Span::styled(" Cancel", raw::form_hint()),
    ]));

    let instructions_widget = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .style(raw::form_hint());

    f.render_widget(title, chunks[chunk_idx]);
    chunk_idx += 1;

    f.render_widget(backend_type_field, chunks[chunk_idx]);
    chunk_idx += 1;

    f.render_widget(name_field, chunks[chunk_idx]);
    chunk_idx += 1;

    f.render_widget(host_field, chunks[chunk_idx]);
    chunk_idx += 1;

    f.render_widget(port_field, chunks[chunk_idx]);
    chunk_idx += 1;

    f.render_widget(password_field, chunks[chunk_idx]);
    chunk_idx += 1;

    if show_database {
        f.render_widget(database_field, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    f.render_widget(instructions_widget, chunks[chunk_idx]);
    chunk_idx += 1;

    // Render error if present
    if let Some(err) = error {
        let error_widget = Paragraph::new(err)
            .style(raw::form_error())
            .alignment(Alignment::Center);
        f.render_widget(error_widget, chunks[chunk_idx]);
    }

    // Render cursor for active input fields
    match form.active_field {
        FormField::Name => {
            let cursor_x = chunks[2].x + 1 + (form.name.visual_cursor() as u16);
            let cursor_y = chunks[2].y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::Host => {
            let cursor_x = chunks[3].x + 1 + (form.host.visual_cursor() as u16);
            let cursor_y = chunks[3].y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::Port => {
            let cursor_x = chunks[4].x + 1 + (form.port.visual_cursor() as u16);
            let cursor_y = chunks[4].y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::Password => {
            // For password, cursor position matches the actual cursor (even though we show masked text)
            let cursor_x = chunks[5].x + 1 + (form.password.visual_cursor() as u16);
            let cursor_y = chunks[5].y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::Database if show_database => {
            let cursor_x = chunks[6].x + 1 + (form.database.visual_cursor() as u16);
            let cursor_y = chunks[6].y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        _ => {}
    }
}
