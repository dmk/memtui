pub mod connection_form;
pub mod connection_list;
pub mod debug;
pub mod help;
pub mod key_list;
pub mod modal;
pub mod status_bar;
pub mod value_viewer;
pub mod warning_message;
pub mod welcome;

use ratatui::{layout::Rect, Frame};

use crate::action::Action;
use crate::events::{Event, EventType};

/// Base trait for UI components
///
/// Components are stateful UI elements that can:
/// - Subscribe to specific event types
/// - Handle events and emit actions
/// - Render themselves
/// - Track their rendered area
pub trait Component {
    /// Data required to render the component (read-only)
    type Props<'a>;

    /// Event types this component wants to receive
    ///
    /// Return the event types this component should be subscribed to.
    /// Global events (Escape, Quit) are delivered via EventType::Global.
    fn subscriptions(&self) -> Vec<EventType> {
        vec![]
    }

    /// Handle an event and optionally emit actions
    ///
    /// The event includes context about focus, mouse position, etc.
    /// Components should check `event.context.focused_component` before
    /// handling non-global events.
    fn handle_event(&mut self, _event: &Event, _props: Self::Props<'_>) -> Vec<Action> {
        vec![]
    }

    /// Render the component
    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>);

    /// Get the last rendered area (for hit-testing and focus management)
    fn area(&self) -> Option<Rect> {
        None
    }
}
