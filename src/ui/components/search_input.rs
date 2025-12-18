//! SearchInput component for handling search mode text input
//!
//! This component handles keyboard input when the user is actively typing in the search box.
//! It captures raw text input (characters, backspace) and delegates command keys to the
//! keybindings system.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::events::{Event, EventKind, EventType};
use crate::keybindings::{BindingContext, KeybindingsConfig};

use super::Component;

/// Props for SearchInput component
pub struct SearchInputProps<'a> {
    /// Reference to keybindings config for command lookup
    pub keybindings: &'a KeybindingsConfig,
    /// Whether search mode is currently active (user is typing)
    pub is_searching: bool,
}

/// SearchInput component handles text input during search mode
///
/// When search mode is active, this component:
/// - Captures printable characters as `SearchAddChar`
/// - Captures Backspace as `SearchDeleteChar`
/// - Delegates other keys (Escape, Enter, arrows) to keybindings
#[derive(Debug, Default)]
pub struct SearchInput;

impl SearchInput {
    pub fn new() -> Self {
        Self
    }

    /// Handle a keybinding command and return the corresponding action(s)
    fn handle_command(&self, command: &str) -> Option<Vec<Action>> {
        match command {
            "search.clear" => Some(vec![Action::ClearSearch]),
            "search.confirm" => Some(vec![Action::ConfirmSearch]),
            "search.next_result" => Some(vec![Action::SearchNextResult]),
            "search.prev_result" => Some(vec![Action::SearchPrevResult]),
            _ => None,
        }
    }
}

impl Component for SearchInput {
    type Props<'a> = SearchInputProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        // Only handle events when search mode is active
        if !props.is_searching {
            return vec![];
        }

        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // First check for keybinding commands (Escape, Enter, arrows for navigation)
        // These take precedence over text input
        if let Some(command) = props.keybindings.get_command(*key, BindingContext::Search) {
            if let Some(actions) = self.handle_command(&command) {
                return actions;
            }
        }

        // Handle text input - characters without control/alt modifiers
        match key.code {
            KeyCode::Backspace => {
                return vec![Action::SearchDeleteChar];
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                return vec![Action::SearchAddChar(c)];
            }
            _ => {}
        }

        vec![]
    }

    fn render(
        &mut self,
        _f: &mut ratatui::Frame,
        _area: ratatui::layout::Rect,
        _props: Self::Props<'_>,
    ) {
        // SearchInput doesn't render anything directly - the search UI is handled
        // by KeyList component which displays the search input inline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventContext;
    use crate::keybindings::default_keybindings;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event {
            kind: EventKind::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            }),
            context: EventContext::default(),
        }
    }

    #[test]
    fn test_char_input_when_searching() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        let event = make_key_event(KeyCode::Char('a'), KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::SearchAddChar('a')));
    }

    #[test]
    fn test_backspace_when_searching() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        let event = make_key_event(KeyCode::Backspace, KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::SearchDeleteChar));
    }

    #[test]
    fn test_escape_clears_search() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        let event = make_key_event(KeyCode::Esc, KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ClearSearch));
    }

    #[test]
    fn test_enter_confirms_search() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        let event = make_key_event(KeyCode::Enter, KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ConfirmSearch));
    }

    #[test]
    fn test_down_navigates_next() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        let event = make_key_event(KeyCode::Down, KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::SearchNextResult));
    }

    #[test]
    fn test_ignores_input_when_not_searching() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: false,
        };

        let event = make_key_event(KeyCode::Char('a'), KeyModifiers::empty());
        let actions = search_input.handle_event(&event, props);

        assert!(actions.is_empty());
    }

    #[test]
    fn test_ignores_ctrl_chars() {
        let mut search_input = SearchInput::new();
        let keybindings = default_keybindings();
        let props = SearchInputProps {
            keybindings: &keybindings,
            is_searching: true,
        };

        // Ctrl+C should not produce SearchAddChar
        let event = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let actions = search_input.handle_event(&event, props);

        // Should be empty since Ctrl+C isn't mapped to a search command
        assert!(actions.is_empty());
    }
}
