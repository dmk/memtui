//! Event system with pub/sub and focus context
//!
//! Components subscribe to event types they care about, and each event
//! carries context (focus, mouse position, etc.) so components can decide
//! whether to handle it.

mod bus;
mod types;

pub use bus::{process_raw_event, spawn_event_poller, EventBus, RawEvent};
pub use types::{ComponentId, Event, EventContext, EventKind, EventType};
