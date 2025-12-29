//! CmdLine component for handling command line text input
//!
//! This component handles keyboard input when the user is actively typing in the command line.
//! It captures raw text input (characters, backspace) and delegates command keys to the
//! keybindings system.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::action::{Action, CmdLineMode};
use crate::events::{Event, EventKind, EventType};
use crate::keybindings::{BindingContext, KeybindingsConfig};
use crate::ui::theme;

use super::Component;

/// Props for CmdLine component
pub struct CmdLineProps<'a> {
    /// Reference to keybindings config for command lookup
    pub keybindings: &'a KeybindingsConfig,
    /// Current command line mode (None = inactive)
    pub mode: Option<CmdLineMode>,
    /// Whether the cmdline is currently focused for input
    pub is_active: bool,
    /// The current buffer content
    pub buffer: &'a str,
    /// Cursor position in the buffer
    pub cursor: usize,
    /// Number of search results found (for search mode info)
    pub search_result_count: Option<usize>,
    /// Whether server search is in progress
    pub is_server_searching: bool,
}

/// CmdLine component handles text input during command line mode
///
/// When command line mode is active, this component:
/// - Captures printable characters as `CmdLineAddChar`
/// - Captures Backspace as `CmdLineDeleteChar`
/// - Delegates other keys (Escape, Enter, arrows) to keybindings
#[derive(Debug, Default)]
pub struct CmdLine;

impl CmdLine {
    pub fn new() -> Self {
        Self
    }

    /// Handle a keybinding command and return the corresponding action(s)
    fn handle_command(&self, command: &str) -> Option<Vec<Action>> {
        match command {
            "cmdline.clear" => Some(vec![Action::CmdLineClear]),
            "cmdline.confirm" => Some(vec![Action::CmdLineConfirm]),
            "cmdline.next_result" => Some(vec![Action::CmdLineNextResult]),
            "cmdline.prev_result" => Some(vec![Action::CmdLinePrevResult]),
            _ => None,
        }
    }

    /// Get the mode prefix character
    fn mode_prefix(mode: CmdLineMode) -> &'static str {
        match mode {
            CmdLineMode::Search => "/",
            CmdLineMode::Command => ":",
            CmdLineMode::Goto => "g ",
            CmdLineMode::Raw => "> ",
        }
    }
}

impl Component for CmdLine {
    type Props<'a> = CmdLineProps<'a>;

    fn subscriptions(&self) -> Vec<EventType> {
        vec![EventType::Key]
    }

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        // Only handle events when command line mode is active
        if props.mode.is_none() {
            return vec![];
        }

        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // First check for keybinding commands (Escape, Enter, arrows for navigation)
        // These take precedence over text input
        if let Some(command) = props.keybindings.get_command(*key, BindingContext::CmdLine) {
            if let Some(actions) = self.handle_command(&command) {
                return actions;
            }
        }

        // Handle text input - characters without control/alt modifiers
        match key.code {
            KeyCode::Backspace => {
                return vec![Action::CmdLineDeleteChar];
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                return vec![Action::CmdLineAddChar(c)];
            }
            _ => {}
        }

        vec![]
    }

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let Some(mode) = props.mode else {
            return;
        };

        let prefix = Self::mode_prefix(mode);
        let prefix_len = prefix.chars().count();

        // Build the command line content with cursor
        let before_cursor: String = props.buffer.chars().take(props.cursor).collect();
        let cursor_char = props.buffer.chars().nth(props.cursor).unwrap_or(' ');
        let after_cursor: String = props.buffer.chars().skip(props.cursor + 1).collect();

        // Build right-side info (result count for search mode)
        let right_info = match mode {
            CmdLineMode::Search => {
                if props.is_server_searching {
                    " searching...".to_string()
                } else if let Some(count) = props.search_result_count {
                    if props.buffer.is_empty() {
                        String::new()
                    } else {
                        format!(" {} found", count)
                    }
                } else {
                    String::new()
                }
            }
            CmdLineMode::Command => String::new(),
            CmdLineMode::Goto => String::new(),
            CmdLineMode::Raw => String::new(),
        };

        // Calculate available width for command line
        let total_width = area.width as usize;
        let right_len = right_info.chars().count();
        let _buffer_max_width = total_width.saturating_sub(prefix_len + right_len + 2);

        // Build spans
        let mut spans = vec![
            // Mode prefix
            Span::styled(prefix, Style::default().fg(theme::NEON_CYAN())),
        ];

        if props.is_active {
            // Buffer content before cursor
            spans.push(Span::styled(
                before_cursor,
                Style::default().fg(theme::TEXT_PRIMARY()),
            ));

            // Cursor (inverted colors)
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default()
                    .fg(theme::BG_DEEP())
                    .bg(theme::TEXT_PRIMARY()),
            ));

            // Buffer content after cursor
            spans.push(Span::styled(
                after_cursor,
                Style::default().fg(theme::TEXT_PRIMARY()),
            ));
        } else {
            spans.push(Span::styled(
                props.buffer,
                Style::default().fg(theme::TEXT_PRIMARY()),
            ));
        }

        // Padding to push right info to the right
        let content_len = prefix_len + props.buffer.len() + if props.is_active { 1 } else { 0 };
        let padding = total_width.saturating_sub(content_len + right_len + 1);
        spans.push(Span::raw(" ".repeat(padding)));

        // Right side info
        if !right_info.is_empty() {
            spans.push(Span::styled(
                right_info,
                Style::default().fg(theme::TEXT_DIM()),
            ));
        }

        // Trailing space
        spans.push(Span::raw(" "));

        let cmdline = Paragraph::new(Line::from(spans)).style(theme::util_bg());
        f.render_widget(cmdline, area);
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

    fn test_props(active: bool) -> CmdLineProps<'static> {
        static KEYBINDINGS: std::sync::OnceLock<KeybindingsConfig> = std::sync::OnceLock::new();
        let keybindings = KEYBINDINGS.get_or_init(default_keybindings);

        CmdLineProps {
            keybindings,
            mode: if active {
                Some(CmdLineMode::Search)
            } else {
                None
            },
            is_active: active,
            buffer: "",
            cursor: 0,
            search_result_count: None,
            is_server_searching: false,
        }
    }

    #[test]
    fn test_char_input_when_active() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        let actions = cmdline.handle_event(&event("a"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CmdLineAddChar('a')));
    }

    #[test]
    fn test_backspace_when_active() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        let actions = cmdline.handle_event(&event("backspace"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CmdLineDeleteChar));
    }

    #[test]
    fn test_escape_clears_cmdline() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        let actions = cmdline.handle_event(&event("esc"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CmdLineClear));
    }

    #[test]
    fn test_enter_confirms_cmdline() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        let actions = cmdline.handle_event(&event("enter"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CmdLineConfirm));
    }

    #[test]
    fn test_down_navigates_next() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        let actions = cmdline.handle_event(&event("down"), props);

        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::CmdLineNextResult));
    }

    #[test]
    fn test_ignores_input_when_not_active() {
        let mut cmdline = CmdLine::new();
        let props = test_props(false);

        let actions = cmdline.handle_event(&event("a"), props);

        assert!(actions.is_empty());
    }

    #[test]
    fn test_ignores_ctrl_chars() {
        let mut cmdline = CmdLine::new();
        let props = test_props(true);

        // Ctrl+C should not produce CmdLineAddChar
        let actions = cmdline.handle_event(&event("ctrl+c"), props);

        // Should be empty since Ctrl+C isn't mapped to a cmdline command
        assert!(actions.is_empty());
    }

    #[test]
    fn test_mode_prefix() {
        assert_eq!(CmdLine::mode_prefix(CmdLineMode::Search), "/");
        assert_eq!(CmdLine::mode_prefix(CmdLineMode::Command), ":");
        assert_eq!(CmdLine::mode_prefix(CmdLineMode::Goto), "g ");
        assert_eq!(CmdLine::mode_prefix(CmdLineMode::Raw), "> ");
    }
}
