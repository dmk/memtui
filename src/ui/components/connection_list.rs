use super::Component;
use crate::action::Action;
use crate::app::ConnectionStatus;
use crate::types::ConnectionConfig;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub struct ConnectionListProps<'a> {
    pub configs: Vec<&'a ConnectionConfig>,
    pub active_id: Option<&'a str>,
    pub statuses: &'a std::collections::HashMap<String, ConnectionStatus>,
    pub is_active: bool,
}

pub struct ConnectionList {
    pub state: ListState,
}

impl ConnectionList {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
    }

    pub fn next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Default for ConnectionList {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ConnectionList {
    type Props<'a> = ConnectionListProps<'a>;
    type Msg = Action;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let connections: Vec<ListItem> = props
            .configs
            .iter()
            .map(|config| {
                let status = props
                    .statuses
                    .get(&config.id)
                    .cloned()
                    .unwrap_or(ConnectionStatus::Disconnected);
                let status_indicator = match status {
                    ConnectionStatus::Connected => "●",
                    ConnectionStatus::Connecting => "◐",
                    ConnectionStatus::Disconnected => "○",
                    ConnectionStatus::Error(_) => "✗",
                };
                let text = format!(
                    "{} {} ({}:{})",
                    status_indicator, config.name, config.host, config.port
                );
                ListItem::new(text)
            })
            .collect();

        let connections_list = List::new(connections)
            .block(
                Block::default()
                    .title("Connections")
                    .borders(Borders::ALL)
                    .border_style(if props.is_active {
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

        f.render_stateful_widget(connections_list, area, &mut self.state);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        // Input handling is done by App via Actions in this architecture
        None
    }
}
