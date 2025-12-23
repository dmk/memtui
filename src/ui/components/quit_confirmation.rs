//! QuitConfirmation component for handling quit confirmation dialog
//!
//! This component renders the quit confirmation modal and handles keyboard input
//! through the keybindings system.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::action::Action;
use crate::events::{Event, EventKind, EventType};
use crate::keybindings::{BindingContext, KeybindingsConfig};
use crate::ui::components::modal::confirm_block_raw;
use crate::ui::theme::raw;

use super::Component;

/// Props for QuitConfirmation component
pub struct QuitConfirmationProps<'a> {
    /// Reference to keybindings config for command lookup
    pub keybindings: &'a KeybindingsConfig,
}

/// QuitConfirmation component handles the quit confirmation dialog
///
/// This component:
/// - Renders a centered confirmation modal
/// - Handles Y/Enter for confirm, N/Esc for cancel via keybindings
#[derive(Debug, Default)]
pub struct QuitConfirmation;

impl QuitConfirmation {
    pub fn new() -> Self {
        Self
    }

    /// Handle a keybinding command and return the corresponding action(s)
    fn handle_command(&self, command: &str) -> Option<Vec<Action>> {
        match command {
            "quit.confirm" => Some(vec![Action::ConfirmQuit]),
            "quit.cancel" => Some(vec![Action::CancelQuit]),
            _ => None,
        }
    }
}

impl Component for QuitConfirmation {
    type Props<'a> = QuitConfirmationProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // Check keybindings for quit confirmation commands
        if let Some(command) = props
            .keybindings
            .get_command(*key, BindingContext::QuitConfirmation)
        {
            if let Some(actions) = self.handle_command(&command) {
                return actions;
            }
        }

        vec![]
    }

    fn render(&mut self, f: &mut Frame, _area: Rect, _props: Self::Props<'_>) {
        // Calculate centered area for the dialog
        let area = centered_rect(45, 25, f.area());
        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width.clamp(35, 55),
            height: area.height.clamp(6, 8),
        };
        let area = Rect {
            x: (f.area().width.saturating_sub(area.width)) / 2,
            y: (f.area().height.saturating_sub(area.height)) / 2,
            width: area.width,
            height: area.height,
        };

        // Use raw (undimmed) colors for modal content
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Are you sure you want to quit?",
                Style::default()
                    .fg(raw::TEXT_BRIGHT())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " y ",
                    Style::default()
                        .fg(raw::BG_DEEP())
                        .bg(raw::NEON_GREEN())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" yes  ", Style::default().fg(raw::TEXT_SECONDARY())),
                Span::styled(
                    " n ",
                    Style::default()
                        .fg(raw::BG_DEEP())
                        .bg(raw::NEON_RED())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" no ", Style::default().fg(raw::TEXT_SECONDARY())),
            ]),
        ];

        let dialog = Paragraph::new(text)
            .block(confirm_block_raw("Quit"))
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(dialog, area);
    }
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width.saturating_sub(popup_width)) / 2;
    let popup_y = (r.height.saturating_sub(popup_height)) / 2;
    Rect {
        x: r.x + popup_x,
        y: r.y + popup_y,
        width: popup_width,
        height: popup_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventContext;
    use crate::keybindings::default_keybindings;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
    fn test_y_confirms_quit() {
        let mut quit_confirm = QuitConfirmation::new();
        let keybindings = default_keybindings();
        let props = QuitConfirmationProps {
            keybindings: &keybindings,
        };

        let event = make_key_event(KeyCode::Char('y'), KeyModifiers::empty());
        let actions = quit_confirm.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ConfirmQuit));
    }

    #[test]
    fn test_enter_confirms_quit() {
        let mut quit_confirm = QuitConfirmation::new();
        let keybindings = default_keybindings();
        let props = QuitConfirmationProps {
            keybindings: &keybindings,
        };

        let event = make_key_event(KeyCode::Enter, KeyModifiers::empty());
        let actions = quit_confirm.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ConfirmQuit));
    }

    #[test]
    fn test_n_cancels_quit() {
        let mut quit_confirm = QuitConfirmation::new();
        let keybindings = default_keybindings();
        let props = QuitConfirmationProps {
            keybindings: &keybindings,
        };

        let event = make_key_event(KeyCode::Char('n'), KeyModifiers::empty());
        let actions = quit_confirm.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CancelQuit));
    }

    #[test]
    fn test_esc_cancels_quit() {
        let mut quit_confirm = QuitConfirmation::new();
        let keybindings = default_keybindings();
        let props = QuitConfirmationProps {
            keybindings: &keybindings,
        };

        let event = make_key_event(KeyCode::Esc, KeyModifiers::empty());
        let actions = quit_confirm.handle_event(&event, props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CancelQuit));
    }

    #[test]
    fn test_other_keys_ignored() {
        let mut quit_confirm = QuitConfirmation::new();
        let keybindings = default_keybindings();
        let props = QuitConfirmationProps {
            keybindings: &keybindings,
        };

        let event = make_key_event(KeyCode::Char('x'), KeyModifiers::empty());
        let actions = quit_confirm.handle_event(&event, props);

        assert!(actions.is_empty());
    }
}
