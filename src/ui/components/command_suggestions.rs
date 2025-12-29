//! CmdLine suggestions popup for command/goto modes
//!
//! Shows available commands for `:` and key suggestions for `g`.

use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::action::Action;
use crate::events::{Event, EventKind, EventType};
use crate::types::KeyMetadata;
use crate::ui::theme;

use super::Component;

/// A command with optional aliases
#[derive(Debug, Clone, Copy)]
pub struct Command {
    /// Primary command name (used for matching and execution)
    pub name: &'static str,
    /// Short alias (displayed first if present)
    pub alias: Option<&'static str>,
    /// Description shown in the popup
    pub description: &'static str,
    /// Whether the command expects arguments (used for insertion)
    pub takes_args: bool,
}

impl Command {
    const fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            alias: None,
            description,
            takes_args: false,
        }
    }

    const fn with_alias(mut self, alias: &'static str) -> Self {
        self.alias = Some(alias);
        self
    }

    const fn with_args(mut self) -> Self {
        self.takes_args = true;
        self
    }

    /// Check if this command matches the given prefix
    fn matches(&self, prefix: &str) -> bool {
        let prefix_lower = prefix.to_lowercase();
        self.name.starts_with(&prefix_lower)
            || self
                .alias
                .map(|a| a.starts_with(&prefix_lower))
                .unwrap_or(false)
    }

    /// Get the display string for the command (e.g., ":q, :quit")
    fn display_name(&self) -> String {
        match self.alias {
            Some(alias) => format!(":{}, :{}", alias, self.name),
            None => format!(":{}", self.name),
        }
    }

    /// Get the shortest form of the command (for insertion)
    pub fn shortest(&self) -> &'static str {
        self.alias.unwrap_or(self.name)
    }

    /// Get the text to insert when selected
    pub fn insert_text(&self) -> String {
        let shortest = self.shortest();
        if self.takes_args {
            format!("{} ", shortest)
        } else {
            shortest.to_string()
        }
    }

    fn suggestion_item(&self) -> SuggestionItem {
        SuggestionItem {
            display: self.display_name(),
            description: Some(self.description.to_string()),
            insert_text: self.insert_text(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SuggestionItem {
    pub display: String,
    pub description: Option<String>,
    pub insert_text: String,
}

/// Available commands
pub const COMMANDS: &[Command] = &[
    Command::new("quit", "quit application").with_alias("q"),
    Command::new("help", "toggle help screen").with_alias("h"),
    Command::new("get", "jump to key by name")
        .with_alias("g")
        .with_args(),
    Command::new("delete", "delete key by name")
        .with_alias("del")
        .with_args(),
    Command::new("raw", "execute raw backend command").with_args(),
];

/// Filter commands by prefix, returning matching suggestions
pub fn command_suggestions(prefix: &str) -> Vec<SuggestionItem> {
    if prefix.is_empty() {
        return COMMANDS.iter().map(Command::suggestion_item).collect();
    }
    COMMANDS
        .iter()
        .filter(|cmd| cmd.matches(prefix))
        .map(Command::suggestion_item)
        .collect()
}

/// Suggest cached keys for goto mode
pub fn goto_suggestions(prefix: &str, keys: &[Option<KeyMetadata>]) -> Vec<SuggestionItem> {
    const MAX_SUGGESTIONS: usize = 100;
    if prefix.is_empty() {
        return Vec::new();
    }

    let mut items = Vec::new();
    for key in keys.iter().filter_map(|entry| entry.as_ref()) {
        if key.name.starts_with(prefix) {
            items.push(SuggestionItem {
                display: key.name.clone(),
                description: None,
                insert_text: key.name.clone(),
            });
            if items.len() >= MAX_SUGGESTIONS {
                break;
            }
        }
    }
    items
}

/// Props for CommandSuggestions component
pub struct CommandSuggestionsProps<'a> {
    /// Suggestions to display
    pub items: &'a [SuggestionItem],
    /// Title shown in the popup
    pub title: &'a str,
    /// Currently selected index
    pub selected_index: Option<usize>,
}

/// Command suggestions popup component
#[derive(Debug, Default)]
pub struct CommandSuggestions {
    state: ListState,
}

impl CommandSuggestions {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
        }
    }
}

impl Component for CommandSuggestions {
    type Props<'a> = CommandSuggestionsProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key, EventType::Scroll]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        if props.items.is_empty() {
            return vec![];
        }

        match &event.kind {
            EventKind::Key(key) => {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Down => vec![Action::CmdLineSuggestionsNext],
                    KeyCode::Up => vec![Action::CmdLineSuggestionsPrev],
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        vec![Action::CmdLineSuggestionsNext]
                    }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        vec![Action::CmdLineSuggestionsPrev]
                    }
                    KeyCode::Tab => vec![Action::CmdLineSuggestionsSelect],
                    _ => vec![],
                }
            }
            EventKind::Scroll { delta, .. } => {
                if *delta > 0 {
                    vec![Action::CmdLineSuggestionsNext]
                } else {
                    vec![Action::CmdLineSuggestionsPrev]
                }
            }
            _ => vec![],
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        if props.items.is_empty() {
            return;
        }

        // Clear the area first
        f.render_widget(Clear, area);

        // Update list state with selected index
        self.state.select(props.selected_index);

        // Build list items
        let has_descriptions = props.items.iter().any(|item| item.description.is_some());
        let items: Vec<ListItem> = props
            .items
            .iter()
            .map(|item| {
                let spans = if has_descriptions {
                    let display = format!("{:<14}", item.display);
                    let description = item.description.as_deref().unwrap_or("");
                    vec![
                        Span::styled(
                            display,
                            Style::default()
                                .fg(theme::NEON_CYAN())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(description, Style::default().fg(theme::TEXT_DIM())),
                    ]
                } else {
                    vec![Span::styled(
                        &item.display,
                        Style::default().fg(theme::TEXT_PRIMARY()),
                    )]
                };
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::BORDER_DIM()))
                    .title(props.title)
                    .title_style(Style::default().fg(theme::TEXT_DIM())),
            )
            .highlight_style(
                Style::default()
                    .bg(theme::BG_SELECTED())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        f.render_stateful_widget(list, area, &mut self.state);
    }
}
