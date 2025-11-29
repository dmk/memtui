use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::types::{Auth, BackendType, ConnectionConfig};
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

pub struct ConnectionForm {
    pub name: Input,
    pub host: Input,
    pub port: Input,
    pub password: Input,
    pub database: Input,
    pub active_field: FormField,
    pub backend_type: BackendType,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            name: Input::default(),
            host: Input::default().with_value("localhost".into()),
            port: Input::default().with_value("6379".into()),
            password: Input::default(),
            database: Input::default().with_value("0".into()),
            active_field: FormField::BackendType,
            backend_type: BackendType::Redis,
        }
    }

    pub fn toggle_backend_type(&mut self) {
        self.backend_type = match self.backend_type {
            BackendType::Redis => {
                self.port = Input::default().with_value("11211".into());
                BackendType::Memcached
            }
            BackendType::Memcached => {
                self.port = Input::default().with_value("6379".into());
                BackendType::Redis
            }
            BackendType::Etcd => {
                self.port = Input::default().with_value("6379".into());
                BackendType::Redis
            }
        };
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        match self.active_field {
            FormField::BackendType => {
                if event.code == KeyCode::Char(' ') {
                    self.toggle_backend_type();
                }
            }
            FormField::Name => {
                let _ = self.name.handle_event(&CrosstermEvent::Key(event));
            }
            FormField::Host => {
                let _ = self.host.handle_event(&CrosstermEvent::Key(event));
            }
            FormField::Port => {
                let _ = self.port.handle_event(&CrosstermEvent::Key(event));
            }
            FormField::Password => {
                let _ = self.password.handle_event(&CrosstermEvent::Key(event));
            }
            FormField::Database => {
                let _ = self.database.handle_event(&CrosstermEvent::Key(event));
            }
        }
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
        self.name = Input::default();
        self.host = Input::default().with_value("localhost".into());
        self.backend_type = BackendType::Redis;
        self.port = Input::default().with_value("6379".into());
        self.password = Input::default();
        self.database = Input::default().with_value("0".into());
        self.active_field = FormField::BackendType;
    }
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self::new()
    }
}

pub fn render_connection_form(f: &mut Frame, form: &ConnectionForm, error: Option<&str>) {
    let area = centered_rect(70, 70, f.area());

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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(1)
        .split(area);

    let mut chunk_idx = 0;

    // Title
    let title = Paragraph::new(format!("Add New {} Connection", form.backend_type))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    // Input fields
    let backend_type_style = if form.active_field == FormField::BackendType {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let name_style = if form.active_field == FormField::Name {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let host_style = if form.active_field == FormField::Host {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let port_style = if form.active_field == FormField::Port {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let password_style = if form.active_field == FormField::Password {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let database_style = if form.active_field == FormField::Database {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let backend_type_display = format!(
        " {} (press Space to toggle)",
        match form.backend_type {
            BackendType::Redis => "Redis",
            BackendType::Memcached => "Memcached",
            BackendType::Etcd => "etcd",
        }
    );

    let backend_type_field = Paragraph::new(backend_type_display)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Backend Type")
                .borders(Borders::ALL)
                .border_style(backend_type_style),
        );

    // Use tui-input for name
    let name_scroll = form
        .name
        .visual_scroll((chunks[chunk_idx].width.max(2) - 2) as usize);
    let name_field = Paragraph::new(form.name.value())
        .scroll((0, name_scroll as u16))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Name (required)")
                .borders(Borders::ALL)
                .border_style(name_style),
        );

    let host_scroll = form
        .host
        .visual_scroll((chunks[chunk_idx].width.max(2) - 2) as usize);
    let host_field = Paragraph::new(form.host.value())
        .scroll((0, host_scroll as u16))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Host")
                .borders(Borders::ALL)
                .border_style(host_style),
        );

    let port_scroll = form
        .port
        .visual_scroll((chunks[chunk_idx].width.max(2) - 2) as usize);
    let port_field = Paragraph::new(form.port.value())
        .scroll((0, port_scroll as u16))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Port")
                .borders(Borders::ALL)
                .border_style(port_style),
        );

    let password_val = form.password.value();
    let password_display = if password_val.is_empty() {
        String::new()
    } else {
        "*".repeat(password_val.len())
    };
    // Password scrolling is tricky with masking, for simplicity we might not scroll perfectly or just show end
    // But tui-input doesn't support masked value out of box for visual_scroll easily unless we implement it.
    // For now, just show masked string.
    let password_field = Paragraph::new(password_display)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Password (optional)")
                .borders(Borders::ALL)
                .border_style(password_style),
        );

    let database_scroll = form
        .database
        .visual_scroll((chunks[chunk_idx].width.max(2) - 2) as usize);
    let database_field = Paragraph::new(form.database.value())
        .scroll((0, database_scroll as u16))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Database")
                .borders(Borders::ALL)
                .border_style(database_style),
        );

    let mut instructions = vec![Line::from("")];

    if matches!(form.backend_type, BackendType::Memcached) {
        instructions.push(Line::from(vec![Span::styled(
            "Note: Memcached doesn't use database field or authentication",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    instructions.push(Line::from(vec![
        Span::styled("Tab/Shift+Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" - Navigate  "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" - Toggle  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" - Save  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" - Cancel"),
    ]));

    let instructions_widget = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    // Render everything
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(Clear, area);
    f.render_widget(block, area);

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
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(error_widget, chunks[chunk_idx]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
