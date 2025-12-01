use super::Component;
use crate::action::Action;
use crate::app::ConnectionStatus;
use crate::types::ConnectionConfig;
use crate::ui::theme::{self, AnimationState};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};

pub struct ConnectionListProps<'a> {
    pub configs: Vec<&'a ConnectionConfig>,
    pub active_id: Option<&'a str>,
    pub statuses: &'a std::collections::HashMap<String, ConnectionStatus>,
    pub is_active: bool,
    pub animation: &'a AnimationState,
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

                let (status_indicator, status_color) = match status {
                    ConnectionStatus::Connected => {
                        (theme::INDICATOR_CONNECTED, theme::NEON_GREEN())
                    }
                    ConnectionStatus::Connecting => {
                        (theme::spinner_pulse(props.animation), theme::NEON_AMBER())
                    }
                    ConnectionStatus::Disconnected => {
                        (theme::INDICATOR_DISCONNECTED, theme::TEXT_DIM())
                    }
                    ConnectionStatus::Error(_) => (theme::INDICATOR_ERROR, theme::NEON_RED()),
                };

                let is_active = props.active_id.map(|id| id == config.id).unwrap_or(false);

                let name_style = if is_active {
                    Style::default()
                        .fg(theme::NEON_CYAN())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_PRIMARY())
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", status_indicator),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(&config.name, name_style),
                    Span::styled(
                        format!(" ({}:{})", config.host, config.port),
                        Style::default().fg(theme::TEXT_DIM()),
                    ),
                ]))
            })
            .collect();

        let connections_list = List::new(connections)
            .block(theme::panel_block("Connections", props.is_active))
            .highlight_style(theme::list_selected().add_modifier(Modifier::BOLD))
            .highlight_symbol("▸ ");

        f.render_stateful_widget(connections_list, area, &mut self.state);
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        // Input handling is done by App via Actions in this architecture
        None
    }
}
