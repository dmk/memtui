use super::Component;
use crate::action::Action;
use crate::types::ConnectionConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

pub struct WelcomeScreenProps<'a> {
    pub recent_configs: Vec<&'a ConnectionConfig>,
}

pub struct WelcomeScreen {
    pub state: ListState,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        let mut state = ListState::default();
        // We don't select by default until user interacts or if we want to highlight first
        // The original code selected the first one if available.
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
        let card_area = Self::centered_rect(70, 70, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(card_area);
        f.render_widget(block, card_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header
                Constraint::Min(0),    // Recent connections list
                Constraint::Length(2), // Footer hints
            ])
            .margin(1)
            .split(inner_area);

        // 1. Header
        let header_text = vec![
            Line::from(Span::styled(
                "Welcome to memtui",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("Browse keys once you connect to a datastore."),
        ];
        let header = Paragraph::new(header_text).alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        // 2. Recent connections list
        if props.recent_configs.is_empty() {
            let no_recent = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No recent connections yet.",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .alignment(Alignment::Center);
            f.render_widget(no_recent, chunks[1]);
        } else {
            // Title for the list
            let title = Paragraph::new(Line::from(Span::styled(
                "Recent connections",
                Style::default()
                    .fg(Color::Yellow)
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
                    let details =
                        format!("{}:{} ({})", config.host, config.port, config.backend_type);

                    // Calculate padding
                    // -2 for borders/padding roughly, maybe more depending on highlight symbol
                    let available_width = list_width.saturating_sub(4);
                    let padding_len = available_width
                        .saturating_sub(name.len())
                        .saturating_sub(details.len());

                    let padding = " ".repeat(padding_len);

                    let content = Line::from(vec![
                        Span::styled(
                            name,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(padding),
                        Span::styled(details, Style::default().fg(Color::DarkGray)),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 30, 34))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");

            f.render_stateful_widget(list, list_area_chunks[1], &mut self.state);
        }

        // 3. Footer
        let footer_text = vec![Line::from(vec![
            Span::styled("Ctrl+P", Style::default().fg(Color::Yellow)),
            Span::raw(" open connections   "),
            Span::styled("Ctrl+N", Style::default().fg(Color::Yellow)),
            Span::raw(" new connection"),
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
                if let Some(idx) = self.state.selected()
                    && let Some(config) = props.recent_configs.get(idx)
                {
                    return Some(Action::FocusConnection(config.id.clone()));
                }
                None
            }
            _ => None,
        }
    }
}
