//! Input widget - text input field with cursor
//!
//! A simple single-line text input with cursor display.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme;

/// State for a text input
#[derive(Debug, Default, Clone)]
pub struct InputState {
    /// Current input value
    pub value: String,
    /// Cursor position (character index)
    pub cursor: usize,
    /// Whether the input is focused
    pub focused: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self {
            value,
            cursor,
            focused: false,
        }
    }

    /// Insert a character at the cursor position
    pub fn insert(&mut self, c: char) {
        let byte_idx = self
            .value
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len());
        self.value.insert(byte_idx, c);
        self.cursor += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_back(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_idx = self
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let next_byte_idx = self
                .value
                .char_indices()
                .nth(self.cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            self.value.replace_range(byte_idx..next_byte_idx, "");
        }
    }

    /// Delete the character at the cursor (delete key)
    pub fn delete_forward(&mut self) {
        let char_count = self.value.chars().count();
        if self.cursor < char_count {
            let byte_idx = self
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let next_byte_idx = self
                .value
                .char_indices()
                .nth(self.cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            self.value.replace_range(byte_idx..next_byte_idx, "");
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        let char_count = self.value.chars().count();
        if self.cursor < char_count {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Set the value and move cursor to end
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.chars().count();
    }
}

/// Style configuration for Input
#[derive(Debug, Clone, Default)]
pub struct InputStyle {
    /// Placeholder text when empty
    pub placeholder: Option<String>,
    /// Whether to show cursor
    pub show_cursor: bool,
    /// Cursor character
    pub cursor_char: char,
}

impl InputStyle {
    pub fn new() -> Self {
        Self {
            placeholder: None,
            show_cursor: true,
            cursor_char: '▍',
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }
}

/// A single-line text input widget
pub struct Input<'a> {
    style: InputStyle,
    label: Option<&'a str>,
    is_active: bool,
}

impl<'a> Input<'a> {
    pub fn new() -> Self {
        Self {
            style: InputStyle::new(),
            label: None,
            is_active: false,
        }
    }

    pub fn style(mut self, style: InputStyle) -> Self {
        self.style = style;
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.style.placeholder = Some(text.into());
        self
    }

    /// Render the input field
    pub fn render(self, f: &mut Frame, area: Rect, state: &InputState) {
        let text_style = if self.is_active && state.focused {
            Style::default().fg(theme::TEXT_PRIMARY())
        } else {
            Style::default().fg(theme::TEXT_SECONDARY())
        };

        let content = if state.value.is_empty() {
            if let Some(ref placeholder) = self.style.placeholder {
                Line::from(Span::styled(
                    placeholder.clone(),
                    Style::default()
                        .fg(theme::TEXT_DIM())
                        .add_modifier(Modifier::ITALIC),
                ))
            } else if self.style.show_cursor && state.focused {
                Line::from(Span::styled(
                    self.style.cursor_char.to_string(),
                    Style::default().fg(theme::ACCENT()),
                ))
            } else {
                Line::from("")
            }
        } else if self.style.show_cursor && state.focused {
            // Build text with cursor
            let mut spans = Vec::new();
            let chars: Vec<char> = state.value.chars().collect();

            if state.cursor > 0 {
                let before: String = chars[..state.cursor].iter().collect();
                spans.push(Span::styled(before, text_style));
            }

            spans.push(Span::styled(
                self.style.cursor_char.to_string(),
                Style::default()
                    .fg(theme::ACCENT())
                    .add_modifier(Modifier::SLOW_BLINK),
            ));

            if state.cursor < chars.len() {
                let after: String = chars[state.cursor..].iter().collect();
                spans.push(Span::styled(after, text_style));
            }

            Line::from(spans)
        } else {
            Line::from(Span::styled(state.value.clone(), text_style))
        };

        let widget = if let Some(label) = self.label {
            let label_span = Span::styled(
                format!("{}: ", label),
                Style::default()
                    .fg(theme::TEXT_SECONDARY())
                    .add_modifier(Modifier::BOLD),
            );
            let mut spans = vec![label_span];
            spans.extend(content.spans);
            Paragraph::new(Line::from(spans))
        } else {
            Paragraph::new(content)
        };

        f.render_widget(widget, area);
    }
}

impl Default for Input<'_> {
    fn default() -> Self {
        Self::new()
    }
}
