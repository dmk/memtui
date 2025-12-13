//! Prompt bar widget - single-line prompt just above the status bar.
//!
//! Used for search today; designed to be reusable for other prompt modes later.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme;

use super::InputState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Search,
    Command,
}

pub struct PromptBar<'a> {
    kind: Option<PromptKind>,
    state: &'a InputState,
    placeholder: Option<&'a str>,
    hint: Option<&'a str>,
}

impl<'a> PromptBar<'a> {
    pub fn new(state: &'a InputState) -> Self {
        Self {
            kind: None,
            state,
            placeholder: None,
            hint: None,
        }
    }

    pub fn kind(mut self, kind: Option<PromptKind>) -> Self {
        self.kind = kind;
        self
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    pub fn hint(mut self, text: &'a str) -> Self {
        self.hint = Some(text);
        self
    }

    pub fn render(self, f: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let bg = theme::util_bg();
        let width = area.width as usize;

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(" "));

        let prefix = match self.kind {
            Some(PromptKind::Search) => Some("/"),
            Some(PromptKind::Command) => Some(":"),
            None => None,
        };

        if let Some(prefix) = prefix {
            spans.push(Span::styled(
                prefix,
                Style::default()
                    .fg(theme::ACCENT())
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));

            spans.extend(Self::render_input_spans(
                self.state,
                self.placeholder,
                width.saturating_sub(3), // leading space + prefix + space
            ));
        } else if let Some(hint) = self.hint {
            spans.push(Span::styled(
                hint,
                Style::default()
                    .fg(theme::TEXT_DIM())
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        let used = spans
            .iter()
            .map(|s| s.content.chars().count())
            .sum::<usize>()
            .min(width);
        let pad = width.saturating_sub(used);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }

        let line = Line::from(spans);
        let widget = Paragraph::new(line).style(bg);
        f.render_widget(widget, area);
    }

    fn render_input_spans(
        state: &InputState,
        placeholder: Option<&str>,
        available_width: usize,
    ) -> Vec<Span<'static>> {
        if available_width == 0 {
            return vec![];
        }

        let value_style = if state.focused {
            Style::default().fg(theme::TEXT_PRIMARY())
        } else {
            Style::default().fg(theme::TEXT_SECONDARY())
        };

        let cursor_style = Style::default()
            .fg(theme::ACCENT())
            .add_modifier(Modifier::SLOW_BLINK);
        let cursor_char = '▍';

        if state.value.is_empty() {
            if state.focused {
                let mut spans = vec![Span::styled(cursor_char.to_string(), cursor_style)];
                if let Some(placeholder) = placeholder {
                    spans.push(Span::styled(
                        placeholder.to_string(),
                        Style::default()
                            .fg(theme::TEXT_DIM())
                            .add_modifier(Modifier::ITALIC),
                    ));
                }
                return spans;
            }

            if let Some(placeholder) = placeholder {
                return vec![Span::styled(
                    placeholder.to_string(),
                    Style::default()
                        .fg(theme::TEXT_DIM())
                        .add_modifier(Modifier::ITALIC),
                )];
            }

            return vec![];
        }

        if !state.focused {
            return vec![Span::styled(state.value.clone(), value_style)];
        }

        // Focused: render a simple cursor-insertion view, biased toward keeping the cursor visible.
        let chars: Vec<char> = state.value.chars().collect();
        let cursor = state.cursor.min(chars.len());

        let before = &chars[..cursor];
        let after = &chars[cursor..];

        let remaining = available_width.saturating_sub(1); // cursor takes 1 char
        let mut before_visible = before.len().min(remaining);
        let mut after_visible = if before_visible < remaining {
            after.len().min(remaining - before_visible)
        } else {
            0
        };

        let mut before_cut = before.len() > before_visible;
        let mut after_cut = after.len() > after_visible;

        // Make room for ellipses if we cut content.
        if before_cut && before_visible > 0 {
            before_visible = before_visible.saturating_sub(1);
        }
        if after_cut && after_visible > 0 {
            after_visible = after_visible.saturating_sub(1);
        }

        // Recompute cuts after adjustments.
        before_cut = before.len() > before_visible;
        after_cut = after.len() > after_visible;

        let before_start = before.len().saturating_sub(before_visible);
        let before_str: String = before[before_start..].iter().collect();
        let after_str: String = after[..after_visible].iter().collect();

        let mut spans: Vec<Span<'static>> = Vec::new();

        if before_cut {
            spans.push(Span::styled("…".to_string(), value_style));
        }
        if !before_str.is_empty() {
            spans.push(Span::styled(before_str, value_style));
        }

        spans.push(Span::styled(cursor_char.to_string(), cursor_style));

        if !after_str.is_empty() {
            spans.push(Span::styled(after_str, value_style));
        }
        if after_cut {
            spans.push(Span::styled("…".to_string(), value_style));
        }

        spans
    }
}

