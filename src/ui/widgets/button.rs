//! Button widget - clickable button with various states
//!
//! A styled button that can display different states (normal, focused, pressed).

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme;

/// Button state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Focused,
    Pressed,
    Disabled,
}

/// Style configuration for a Button
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// Normal state colors
    pub fg: ratatui::style::Color,
    pub bg: ratatui::style::Color,
    /// Focused state colors
    pub fg_focused: ratatui::style::Color,
    pub bg_focused: ratatui::style::Color,
    /// Disabled state colors
    pub fg_disabled: ratatui::style::Color,
    pub bg_disabled: ratatui::style::Color,
    /// Minimum width (padding will be added)
    pub min_width: u16,
    /// Alignment
    pub alignment: Alignment,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            fg: theme::BG_DEEP(),
            bg: theme::NEON_CYAN(),
            fg_focused: theme::BG_DEEP(),
            bg_focused: theme::ACCENT_BRIGHT(),
            fg_disabled: theme::TEXT_DIM(),
            bg_disabled: theme::BG_SURFACE(),
            min_width: 8,
            alignment: Alignment::Center,
        }
    }
}

impl ButtonStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn primary() -> Self {
        Self::default()
    }

    pub fn secondary() -> Self {
        Self {
            fg: theme::TEXT_PRIMARY(),
            bg: theme::BG_SURFACE(),
            fg_focused: theme::BG_DEEP(),
            bg_focused: theme::TEXT_SECONDARY(),
            ..Self::default()
        }
    }

    pub fn danger() -> Self {
        Self {
            fg: theme::BG_DEEP(),
            bg: theme::NEON_RED(),
            fg_focused: theme::BG_DEEP(),
            bg_focused: theme::NEON_PINK(),
            ..Self::default()
        }
    }

    pub fn success() -> Self {
        Self {
            fg: theme::BG_DEEP(),
            bg: theme::NEON_GREEN(),
            fg_focused: theme::BG_DEEP(),
            bg_focused: theme::NEON_CYAN(),
            ..Self::default()
        }
    }

    pub fn min_width(mut self, width: u16) -> Self {
        self.min_width = width;
        self
    }
}

/// A clickable button widget
pub struct Button<'a> {
    label: &'a str,
    state: ButtonState,
    style: ButtonStyle,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            state: ButtonState::Normal,
            style: ButtonStyle::default(),
        }
    }

    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = state;
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.state = if focused {
            ButtonState::Focused
        } else {
            ButtonState::Normal
        };
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        if disabled {
            self.state = ButtonState::Disabled;
        }
        self
    }

    /// Render the button
    pub fn render(self, f: &mut Frame, area: Rect) {
        let (fg, bg) = match self.state {
            ButtonState::Normal => (self.style.fg, self.style.bg),
            ButtonState::Focused | ButtonState::Pressed => {
                (self.style.fg_focused, self.style.bg_focused)
            }
            ButtonState::Disabled => (self.style.fg_disabled, self.style.bg_disabled),
        };

        let mut style = Style::default().fg(fg).bg(bg);
        if self.state == ButtonState::Focused {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.state == ButtonState::Disabled {
            style = style.add_modifier(Modifier::DIM);
        }

        // Pad the label to min_width
        let label_len = self.label.chars().count();
        let padded_label = if label_len < self.style.min_width as usize {
            let total_pad = self.style.min_width as usize - label_len;
            let left_pad = total_pad / 2;
            let right_pad = total_pad - left_pad;
            format!(
                "{}{}{}",
                " ".repeat(left_pad),
                self.label,
                " ".repeat(right_pad)
            )
        } else {
            format!(" {} ", self.label)
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(padded_label, style)))
            .alignment(self.style.alignment);

        f.render_widget(paragraph, area);
    }
}
