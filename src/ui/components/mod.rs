pub mod connection_form;
pub mod connection_list;
pub mod help;
pub mod key_browser;
pub mod modal;
pub mod status_bar;
pub mod value_viewer;
pub mod warning_message;
pub mod welcome;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

/// Base trait for UI components
pub trait Component {
    /// Data required to render the component (read-only)
    type Props<'a>;
    /// Events emitted by the component to the parent
    type Msg;

    /// Render the component
    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>);

    /// Handle input events
    /// Returns Option<Msg> if the event triggered an action the parent needs to handle
    fn handle_input(&mut self, key: KeyEvent, props: Self::Props<'_>) -> Option<Self::Msg>;
}
