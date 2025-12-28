//! Event type definitions for the pub/sub system

// Re-export non-generic types directly
pub use tui_dispatch::{EventKind, EventType};

/// Unique identifier for components (memtui-specific variants)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, tui_dispatch::ComponentId)]
pub enum ComponentId {
    KeyList,
    ValueViewer,
    ConnectionPalette,
    ConnectionForm,
    WelcomeScreen,
    StatusBar,
    Help,
    QuitConfirmation,
    SearchInput,
}

/// Event context with memtui's ComponentId
pub type EventContext = tui_dispatch::EventContext<ComponentId>;

/// Event with memtui's ComponentId
pub type Event = tui_dispatch::Event<ComponentId>;
