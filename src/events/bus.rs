//! Event bus for dispatching events to subscribed components

use super::types::ComponentId;
use crate::action::Action;

// Re-export from tui_dispatch
pub use tui_dispatch::{process_raw_event, spawn_event_poller, RawEvent};

/// Event bus type alias with memtui's Action and ComponentId types
pub type EventBus = tui_dispatch::EventBus<Action, ComponentId>;
