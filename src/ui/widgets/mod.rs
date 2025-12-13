//! Reusable UI widgets - pure rendering primitives
//!
//! Widgets are stateless rendering helpers that can be composed
//! to build more complex components.

mod button;
mod input;
mod list;
mod modal;
mod pane;
mod prompt_bar;
mod status_badge;
mod tabs;

pub use button::{Button, ButtonState, ButtonStyle};
pub use input::{Input, InputState, InputStyle};
pub use list::{SelectableList, SelectableListState};
pub use modal::{Modal, ModalStyle};
pub use pane::{Pane, PaneStyle};
pub use prompt_bar::{PromptBar, PromptKind};
pub use status_badge::{StatusBadge, StatusKind};
pub use tabs::{TabBar, TabItem};
