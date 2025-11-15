use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Connections,
    Keys,
    Value,
}

struct App {
    active_panel: Panel,
    show_help: bool,
    connections: Vec<String>,
    connection_state: ListState,
    keys: Vec<String>,
    key_state: ListState,
    selected_value: String,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            active_panel: Panel::Keys,
            show_help: false,
            connections: vec![
                "Redis (localhost:6379)".to_string(),
                "Memcached (localhost:11211)".to_string(),
                "etcd (localhost:2379)".to_string(),
            ],
            connection_state: ListState::default(),
            keys: vec![
                "user:123".to_string(),
                "user:456".to_string(),
                "session:abc".to_string(),
                "session:def".to_string(),
                "cache:config".to_string(),
                "cache:settings".to_string(),
            ],
            key_state: ListState::default(),
            selected_value: String::new(),
        };
        app.key_state.select(Some(0));
        app
    }

    fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Connections => Panel::Keys,
            Panel::Keys => Panel::Value,
            Panel::Value => Panel::Connections,
        };
    }

    fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Connections => Panel::Value,
            Panel::Keys => Panel::Connections,
            Panel::Value => Panel::Keys,
        };
    }

    fn next_item(&mut self) {
        match self.active_panel {
            Panel::Connections => {
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i >= self.connections.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
            }
            Panel::Keys => {
                let i = match self.key_state.selected() {
                    Some(i) => {
                        if i >= self.keys.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.key_state.select(Some(i));
                self.update_value();
            }
            Panel::Value => {}
        }
    }

    fn previous_item(&mut self) {
        match self.active_panel {
            Panel::Connections => {
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.connections.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
            }
            Panel::Keys => {
                let i = match self.key_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.keys.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.key_state.select(Some(i));
                self.update_value();
            }
            Panel::Value => {}
        }
    }

    fn update_value(&mut self) {
        if let Some(i) = self.key_state.selected() {
            if let Some(key) = self.keys.get(i) {
                // Mock values for demonstration
                self.selected_value = match key.as_str() {
                    "user:123" => r#"{"id": 123, "name": "Alice", "email": "alice@example.com"}"#.to_string(),
                    "user:456" => r#"{"id": 456, "name": "Bob", "email": "bob@example.com"}"#.to_string(),
                    "session:abc" => "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
                    "session:def" => "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
                    "cache:config" => r#"{"theme": "dark", "lang": "en"}"#.to_string(),
                    "cache:settings" => r#"{"notifications": true, "timeout": 30}"#.to_string(),
                    _ => "No value".to_string(),
                };
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.update_value();

    // Run app
    let res = run_app(&mut terminal, &mut app);

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

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            ui(f, app);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.show_help {
                    // Close help on any key
                    app.show_help = false;
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('?') => app.show_help = true,
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    // Create three-panel layout
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(f.area());

    // Left panel: Connections
    let connections: Vec<ListItem> = app
        .connections
        .iter()
        .map(|c| ListItem::new(c.as_str()))
        .collect();

    let connections_list = List::new(connections)
        .block(
            Block::default()
                .title("Connections")
                .borders(Borders::ALL)
                .border_style(if app.active_panel == Panel::Connections {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(connections_list, chunks[0], &mut app.connection_state);

    // Middle panel: Key Browser
    let keys: Vec<ListItem> = app
        .keys
        .iter()
        .map(|k| ListItem::new(k.as_str()))
        .collect();

    let keys_list = List::new(keys)
        .block(
            Block::default()
                .title("Key Browser [6 keys]")
                .borders(Borders::ALL)
                .border_style(if app.active_panel == Panel::Keys {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(keys_list, chunks[1], &mut app.key_state);

    // Right panel: Value Viewer
    let value_text = if !app.selected_value.is_empty() {
        app.selected_value.clone()
    } else {
        "Select a key to view its value".to_string()
    };

    let value = Paragraph::new(value_text)
        .block(
            Block::default()
                .title("Value Viewer")
                .borders(Borders::ALL)
                .border_style(if app.active_panel == Panel::Value {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(value, chunks[2]);

    // Show help modal if active
    if app.show_help {
        render_help(f);
    }
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 50, f.area());

    let help_text = vec![
        Line::from(Span::styled(
            "memtui - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab         ", Style::default().fg(Color::Yellow)),
            Span::raw("Next panel"),
        ]),
        Line::from(vec![
            Span::styled("Shift+Tab   ", Style::default().fg(Color::Yellow)),
            Span::raw("Previous panel"),
        ]),
        Line::from(vec![
            Span::styled("↑/k         ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("↓/j         ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("?           ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle help"),
        ]),
        Line::from(vec![
            Span::styled("q           ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    f.render_widget(Clear, area);
    f.render_widget(help, area);
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
