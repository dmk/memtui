//! HelpScreen component for displaying keyboard shortcuts help
//!
//! This component renders the help modal and handles keyboard input
//! through the keybindings system.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::modal::render_modal;
use super::Component;
use crate::action::Action;
use crate::events::{Event, EventKind, EventType};
use crate::keybindings::{format_key_for_display, BindingContext, KeybindingsConfig};
use crate::ui::theme::{self, raw};

/// Props for HelpScreen component
pub struct HelpScreenProps<'a> {
    /// Reference to keybindings config for command lookup and display
    pub keybindings: &'a KeybindingsConfig,
}

/// HelpScreen component displays keyboard shortcuts and handles close events
///
/// This component:
/// - Renders a centered modal with all keyboard shortcuts
/// - Handles close commands via keybindings (?, Esc, q)
#[derive(Debug, Default)]
pub struct HelpScreen;

impl HelpScreen {
    pub fn new() -> Self {
        Self
    }

    /// Handle a keybinding command and return the corresponding action(s)
    fn handle_command(&self, command: &str) -> Option<Vec<Action>> {
        match command {
            "help.close" | "help.toggle" => Some(vec![Action::ToggleHelp]),
            _ => None,
        }
    }
}

impl Component for HelpScreen {
    type Props<'a> = HelpScreenProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // Check keybindings for help commands
        if let Some(command) = props.keybindings.get_command(*key, BindingContext::Help) {
            if let Some(actions) = self.handle_command(&command) {
                return actions;
            }
        }

        vec![]
    }

    fn render(&mut self, f: &mut Frame, _area: Rect, props: Self::Props<'_>) {
        let keybindings = props.keybindings;
        let area = theme::centered_rect(50, 60, f.area());
        let context = BindingContext::Default;

        // Helper to get keybinding
        let get_key = |action: &str, default: &str| -> String {
            keybindings
                .get_first_keybinding(action, context)
                .map(|k| format_key_for_display(&k))
                .unwrap_or_else(|| default.to_string())
        };

        // Get keybindings from config
        let next_panel = get_key("navigation.next_panel", "Tab");
        let prev_panel = get_key("navigation.prev_panel", "⇧Tab");
        let next_item = get_key("navigation.next_item", "↓");
        let prev_item = get_key("navigation.prev_item", "↑");
        let search = get_key("search.start", "/");
        let value_view_mode = get_key("value.view_mode.cycle", "V");
        let palette = get_key("connection.palette.toggle", "^O");
        let new_conn = get_key("connection.form.open", "^N");
        let tab_next = get_key("connection.tab.next", "^→");
        let tab_prev = get_key("connection.tab.prev", "^←");
        let resize_left = get_key("pane.resize.left", "^H");
        let resize_right = get_key("pane.resize.right", "^L");
        let help = get_key("help.toggle", "?");
        let quit = get_key("quit.show", "Q");

        let help_sections: Vec<(&str, Vec<(&str, String)>)> = vec![
            (
                "Navigation",
                vec![
                    ("Switch panels", format!("{}  {}", next_panel, prev_panel)),
                    ("Move up/down", format!("{}  {}", prev_item, next_item)),
                    ("Search keys", search),
                    ("Value view mode", value_view_mode),
                ],
            ),
            (
                "Connections",
                vec![
                    ("Connection palette", palette),
                    ("New connection", new_conn),
                    ("Switch tabs", format!("{}  {}", tab_prev, tab_next)),
                ],
            ),
            (
                "Panes",
                vec![
                    ("Resize panes", format!("{}  {}", resize_left, resize_right)),
                    ("Mouse resize", "drag border".to_string()),
                ],
            ),
            ("General", vec![("Toggle help", help), ("Quit", quit)]),
        ];

        // Render modal with dimmed background
        let block_inner = render_modal(f, area, "Help");

        // Calculate inner area with extra padding
        let inner = Rect {
            x: block_inner.x + 1,
            y: block_inner.y + 1,
            width: block_inner.width.saturating_sub(2),
            height: block_inner.height.saturating_sub(1),
        };

        // Render title centered (use raw colors for modal content)
        let title = Paragraph::new(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(raw::NEON_CYAN())
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let title_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        };
        f.render_widget(title, title_area);

        // Render sections
        let mut y_offset = 2u16;

        for (section_name, bindings) in help_sections {
            // Section header
            let section_y = inner.y + y_offset;
            if section_y >= inner.y + inner.height {
                break;
            }

            let section_header = Paragraph::new(Span::styled(
                section_name,
                Style::default()
                    .fg(raw::NEON_AMBER())
                    .add_modifier(Modifier::BOLD),
            ));
            f.render_widget(
                section_header,
                Rect {
                    x: inner.x + 2,
                    y: section_y,
                    width: inner.width.saturating_sub(4),
                    height: 1,
                },
            );
            y_offset += 1;

            // Bindings
            for (desc, key) in bindings {
                let binding_y = inner.y + y_offset;
                if binding_y >= inner.y + inner.height.saturating_sub(1) {
                    break;
                }

                // Calculate available width for the line
                let line_width = inner.width.saturating_sub(10) as usize;
                let desc_len = desc.chars().count();
                let key_len = key.chars().count();
                let fill_len = line_width.saturating_sub(desc_len + key_len);

                // Create dotted fill using ASCII dots for consistent spacing
                let fill = if fill_len > 2 {
                    format!(" {} ", ".".repeat(fill_len - 2))
                } else {
                    " ".repeat(fill_len)
                };

                // Create the full line: "Description ..... Key"
                let line = Line::from(vec![
                    Span::styled(desc, Style::default().fg(raw::TEXT_DIM())),
                    Span::styled(fill, Style::default().fg(Color::Rgb(55, 60, 70))),
                    Span::styled(&key, Style::default().fg(raw::NEON_CYAN())),
                ]);

                let line_widget = Paragraph::new(line);
                f.render_widget(
                    line_widget,
                    Rect {
                        x: inner.x + 4,
                        y: binding_y,
                        width: inner.width.saturating_sub(6),
                        height: 1,
                    },
                );

                y_offset += 1;
            }
            y_offset += 1; // Space between sections
        }

        // Footer
        let footer_y = inner.y + inner.height.saturating_sub(1);
        let footer = Paragraph::new(Span::styled(
            "Press any key to close",
            Style::default()
                .fg(raw::TEXT_DIM())
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center);
        f.render_widget(
            footer,
            Rect {
                x: inner.x,
                y: footer_y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ComponentId;
    use crate::keybindings::default_keybindings;
    use tui_dispatch::testing::key_event;

    fn event(s: &str) -> Event {
        key_event::<ComponentId>(s)
    }

    #[test]
    fn test_question_mark_closes_help() {
        let mut help_screen = HelpScreen::new();
        let keybindings = default_keybindings();
        let props = HelpScreenProps {
            keybindings: &keybindings,
        };

        let actions = help_screen.handle_event(&event("?"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ToggleHelp));
    }

    #[test]
    fn test_esc_closes_help() {
        let mut help_screen = HelpScreen::new();
        let keybindings = default_keybindings();
        let props = HelpScreenProps {
            keybindings: &keybindings,
        };

        let actions = help_screen.handle_event(&event("esc"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ToggleHelp));
    }

    #[test]
    fn test_q_closes_help() {
        let mut help_screen = HelpScreen::new();
        let keybindings = default_keybindings();
        let props = HelpScreenProps {
            keybindings: &keybindings,
        };

        let actions = help_screen.handle_event(&event("q"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ToggleHelp));
    }

    #[test]
    fn test_other_keys_ignored() {
        let mut help_screen = HelpScreen::new();
        let keybindings = default_keybindings();
        let props = HelpScreenProps {
            keybindings: &keybindings,
        };

        // 'x' is not mapped to help.close
        let actions = help_screen.handle_event(&event("x"), props);

        assert!(actions.is_empty());
    }
}
