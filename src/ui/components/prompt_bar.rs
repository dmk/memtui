//! Prompt bar component - manages prompt input state and rendering.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{layout::Rect, Frame};

use super::Component;
use crate::action::Action;
use crate::events::{Event, EventKind};
use crate::ui::widgets::{InputState, PromptBar, PromptKind};

pub struct PromptBarProps<'a> {
    pub kind: Option<PromptKind>,
    pub value: &'a str,
    pub is_focused: bool,
    pub placeholder: Option<&'a str>,
    pub hint: Option<&'a str>,
}

pub struct PromptBarComponent {
    input: InputState,
    last_area: Option<Rect>,
    was_focused: bool,
}

impl PromptBarComponent {
    pub fn new() -> Self {
        Self {
            input: InputState::default(),
            last_area: None,
            was_focused: false,
        }
    }
}

impl Default for PromptBarComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for PromptBarComponent {
    type Props<'a> = PromptBarProps<'a>;

    fn handle_event(&mut self, event: &Event, props: Self::Props<'_>) -> Vec<Action> {
        if !props.is_focused {
            return vec![];
        }

        let EventKind::Key(key) = &event.kind else {
            return vec![];
        };

        // Only handle direct text editing keys here; other keys are mapped via keybindings.
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert(c);
                vec![Action::UpdateSearchQuery(self.input.value.clone())]
            }
            KeyCode::Backspace => {
                self.input.delete_back();
                vec![Action::UpdateSearchQuery(self.input.value.clone())]
            }
            KeyCode::Delete => {
                self.input.delete_forward();
                vec![Action::UpdateSearchQuery(self.input.value.clone())]
            }
            KeyCode::Left => {
                self.input.move_left();
                vec![Action::Tick]
            }
            KeyCode::Right => {
                self.input.move_right();
                vec![Action::Tick]
            }
            KeyCode::Home => {
                self.input.move_start();
                vec![Action::Tick]
            }
            KeyCode::End => {
                self.input.move_end();
                vec![Action::Tick]
            }
            _ => vec![],
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        self.last_area = Some(area);

        if self.input.value != props.value {
            self.input.set_value(props.value);
        }

        self.input.focused = props.is_focused;

        if props.is_focused && !self.was_focused {
            self.input.move_end();
        }
        self.was_focused = props.is_focused;

        let mut widget = PromptBar::new(&self.input).kind(props.kind);
        if let Some(placeholder) = props.placeholder {
            widget = widget.placeholder(placeholder);
        }
        if let Some(hint) = props.hint {
            widget = widget.hint(hint);
        }
        widget.render(f, area);
    }

    fn area(&self) -> Option<Rect> {
        self.last_area
    }
}

