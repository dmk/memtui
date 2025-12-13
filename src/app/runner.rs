//! Application runner with event-based architecture
//!
//! This module provides the main event loop using the EventBus system.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::action::Action;
use crate::app::AppState;
use crate::config::Config;
use crate::events::{
    process_raw_event, spawn_event_poller, ComponentId, Event, EventBus, EventContext, EventKind,
    RawEvent,
};
use crate::ui::{Component, Panel, UiState};

/// Simplified event loop runner that uses the EventBus
pub struct EventRunner {
    event_bus: EventBus,
    raw_event_rx: mpsc::UnboundedReceiver<RawEvent>,
    cancel_token: CancellationToken,
}

impl EventRunner {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>, config: &Config) -> Self {
        let (raw_tx, raw_rx) = mpsc::unbounded_channel();
        let event_bus = EventBus::new(action_tx);
        let cancel_token = CancellationToken::new();

        // Spawn the event poller with cancel token
        let _poller = spawn_event_poller(
            raw_tx,
            config.performance.event_poll_timeout,
            config.performance.event_loop_sleep,
            cancel_token.clone(),
        );

        Self {
            event_bus,
            raw_event_rx: raw_rx,
            cancel_token,
        }
    }

    /// Get mutable access to the event bus for configuration
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// Update the event context (call after rendering to update component areas)
    pub fn update_context<F>(&mut self, f: F)
    where
        F: FnOnce(&mut EventContext),
    {
        f(self.event_bus.context_mut());
    }

    /// Poll for the next event (non-blocking)
    pub fn try_poll(&mut self) -> Option<Event> {
        match self.raw_event_rx.try_recv() {
            Ok(raw) => {
                let kind = process_raw_event(raw);

                // Update context from event
                match &kind {
                    EventKind::Key(key) => {
                        self.event_bus.update_modifiers(key.modifiers);
                    }
                    EventKind::Mouse(mouse) => {
                        self.event_bus
                            .update_mouse_position(mouse.column, mouse.row);
                    }
                    EventKind::Scroll { column, row, .. } => {
                        self.event_bus.update_mouse_position(*column, *row);
                    }
                    _ => {}
                }

                Some(self.event_bus.create_event(kind))
            }
            Err(_) => None,
        }
    }

    /// Wait for the next event (blocking)
    pub async fn poll(&mut self) -> Option<Event> {
        match self.raw_event_rx.recv().await {
            Some(raw) => {
                let kind = process_raw_event(raw);

                // Update context from event
                match &kind {
                    EventKind::Key(key) => {
                        self.event_bus.update_modifiers(key.modifiers);
                    }
                    EventKind::Mouse(mouse) => {
                        self.event_bus
                            .update_mouse_position(mouse.column, mouse.row);
                    }
                    EventKind::Scroll { column, row, .. } => {
                        self.event_bus.update_mouse_position(*column, *row);
                    }
                    _ => {}
                }

                Some(self.event_bus.create_event(kind))
            }
            None => None,
        }
    }

    /// Cancel the event loop
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Get the cancellation token for external use
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

/// Helper to determine which component is focused based on UI state
pub fn get_focused_component(ui_state: &UiState, app_state: &AppState) -> Option<ComponentId> {
    // Modal components take priority
    if ui_state.show_quit_confirmation {
        return Some(ComponentId::QuitConfirmation);
    }
    if ui_state.show_help {
        return Some(ComponentId::Help);
    }
    if ui_state.show_connection_form {
        return Some(ComponentId::ConnectionForm);
    }
    if ui_state.show_connection_palette {
        return Some(ComponentId::ConnectionPalette);
    }

    // Prompt bar takes focus while active
    if app_state.is_searching {
        return Some(ComponentId::PromptBar);
    }

    // No active connection = welcome screen
    if app_state.connection_manager.get_active_id().is_none() {
        return Some(ComponentId::WelcomeScreen);
    }

    // Otherwise, based on active panel
    match ui_state.active_panel {
        Panel::Keys => Some(ComponentId::KeyList),
        Panel::Value => Some(ComponentId::ValueViewer),
    }
}

/// Helper to determine if a modal is currently open
pub fn is_modal_open(ui_state: &UiState) -> bool {
    ui_state.show_quit_confirmation
        || ui_state.show_help
        || ui_state.show_connection_form
        || ui_state.show_connection_palette
}

/// Helper to get the active modal component
pub fn get_active_modal(ui_state: &UiState) -> Option<ComponentId> {
    if ui_state.show_quit_confirmation {
        Some(ComponentId::QuitConfirmation)
    } else if ui_state.show_help {
        Some(ComponentId::Help)
    } else if ui_state.show_connection_form {
        Some(ComponentId::ConnectionForm)
    } else if ui_state.show_connection_palette {
        Some(ComponentId::ConnectionPalette)
    } else {
        None
    }
}

/// Update event context with current UI state
pub fn sync_event_context(context: &mut EventContext, ui_state: &UiState, app_state: &AppState) {
    context.focused_component = get_focused_component(ui_state, app_state);
    context.is_modal_open = is_modal_open(ui_state);
    context.active_modal = get_active_modal(ui_state);

    // Update component areas
    if let Some(area) = ui_state.last_key_area {
        context.set_component_area(ComponentId::KeyList, area);
    }
    if let Some(area) = ui_state.last_value_area {
        context.set_component_area(ComponentId::ValueViewer, area);
    }
    if let Some(area) = ui_state.connection_palette_area {
        context.set_component_area(ComponentId::ConnectionPalette, area);
    }
    if let Some(area) = ui_state.welcome_screen.last_list_area {
        context.set_component_area(ComponentId::WelcomeScreen, area);
    }
    if let Some(area) = ui_state.prompt_bar.area() {
        context.set_component_area(ComponentId::PromptBar, area);
    }
}
