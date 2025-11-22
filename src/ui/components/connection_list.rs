use super::Component;
use crate::action::Action;
use crate::app::ConnectionStatus;
use crate::types::ConnectionConfig;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
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

    pub fn index_at_position(
        &self,
        area: Rect,
        column: u16,
        row: u16,
        total: usize,
    ) -> Option<usize> {
        if total == 0 || area.height <= 2 || area.width <= 2 {
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

        let rel = (row - inner_top) as usize;
        if rel >= total {
            return None;
        }
        Some(rel)
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

        let highlight_border = if props.is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let connections_list = List::new(connections)
            .block(
                Block::default()
                    .title("Connections")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(highlight_border),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 34))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        f.render_stateful_widget(connections_list, area, &mut self.state);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        // Input handling is done by App via Actions in this architecture
        None
    }
}
