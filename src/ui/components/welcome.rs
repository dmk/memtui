use super::Component;
use crate::action::Action;
use crate::types::ConnectionConfig;
use crate::ui::theme::{self, AnimationState};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct WelcomeScreenProps<'a> {
    pub recent_configs: Vec<&'a ConnectionConfig>,
    pub animation: &'a AnimationState,
}

pub struct WelcomeScreen {
    pub state: ListState,
}

impl WelcomeScreen {
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
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for WelcomeScreen {
    type Props<'a> = WelcomeScreenProps<'a>;
    type Msg = Action;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        // Fill background
        let bg = Block::default().style(Style::default().bg(theme::BG_DEEP()));
        f.render_widget(bg, area);

        let card_area = Self::centered_rect(75, 75, area);

        // Static fancy border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme::NEON_PURPLE()))
            .style(Style::default().bg(theme::BG_PANEL()));

        let inner_area = block.inner(card_area);
        f.render_widget(block, card_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Logo + header
                Constraint::Min(0),    // Recent connections list
                Constraint::Length(3), // Footer hints
            ])
            .margin(1)
            .split(inner_area);

        // 1. Animated Logo Header
        let logo_lines = theme::logo_lines(props.animation);
        let mut header_lines = logo_lines;
        header_lines.push(Line::from(""));
        header_lines.push(Line::from(Span::styled(
            "Terminal UI for Redis & Memcached",
            Style::default().fg(theme::TEXT_SECONDARY()),
        )));

        let header = Paragraph::new(header_lines).alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        // 2. Recent connections list
        if props.recent_configs.is_empty() {
            let no_recent = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No recent connections",
                    Style::default()
                        .fg(theme::TEXT_DIM())
                        .add_modifier(Modifier::ITALIC),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Ctrl+N to create a new connection",
                    Style::default().fg(theme::TEXT_SECONDARY()),
                )),
            ])
            .alignment(Alignment::Center);
            f.render_widget(no_recent, chunks[1]);
        } else {
            // Title for the list
            let title = Paragraph::new(Line::from(Span::styled(
                "Recent Connections",
                Style::default()
                    .fg(theme::NEON_AMBER())
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);

            let list_area_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(0)])
                .split(chunks[1]);

            f.render_widget(title, list_area_chunks[0]);

            let list_width = list_area_chunks[1].width as usize;

            let items: Vec<ListItem> = props
                .recent_configs
                .iter()
                .map(|config| {
                    let name = &config.name;
                    let backend_icon = match config.backend_type {
                        crate::types::BackendType::Redis => "◈",
                        crate::types::BackendType::Memcached => "◇",
                        crate::types::BackendType::Etcd => "◆",
                    };
                    let details =
                        format!("{}:{} ({})", config.host, config.port, config.backend_type);

                    // Calculate padding
                    let available_width = list_width.saturating_sub(6);
                    let content_width = name.len() + details.len() + 3; // +3 for icon and spaces
                    let padding_len = available_width.saturating_sub(content_width);
                    let padding = " ".repeat(padding_len);

                    let content = Line::from(vec![
                        Span::styled(
                            format!("{} ", backend_icon),
                            Style::default().fg(theme::NEON_CYAN()),
                        ),
                        Span::styled(
                            name,
                            Style::default()
                                .fg(theme::TEXT_BRIGHT())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(padding),
                        Span::styled(details, Style::default().fg(theme::TEXT_DIM())),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(theme::list_selected().add_modifier(Modifier::BOLD))
                .highlight_symbol("▸ ");

            f.render_stateful_widget(list, list_area_chunks[1], &mut self.state);
        }

        // 3. Footer with styled keybindings
        let footer_text = vec![Line::from(vec![
            Span::styled(
                " Ctrl+P ",
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::NEON_CYAN())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" connections  ", Style::default().fg(theme::TEXT_SECONDARY())),
            Span::styled(
                " Ctrl+N ",
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::NEON_GREEN())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" new  ", Style::default().fg(theme::TEXT_SECONDARY())),
            Span::styled(
                " ? ",
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::NEON_AMBER())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" help", Style::default().fg(theme::TEXT_SECONDARY())),
        ])];
        let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyEvent, props: Self::Props<'_>) -> Option<Self::Msg> {
        if props.recent_configs.is_empty() {
            return None;
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.next(props.recent_configs.len());
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev(props.recent_configs.len());
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.state.selected() {
                    if let Some(config) = props.recent_configs.get(idx) {
                        return Some(Action::FocusConnection(config.id.clone()));
                    }
                }
                None
            }
            _ => None,
        }
    }
}
