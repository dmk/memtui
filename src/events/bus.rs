//! Event bus for dispatching events to subscribed components

use super::types::{ComponentId, Event, EventContext, EventKind, EventType};
use crate::action::Action;
use crossterm::event::{self, KeyModifiers, MouseEventKind};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;

/// Raw event from crossterm before processing
#[derive(Debug)]
pub enum RawEvent {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    Resize(u16, u16),
}

/// Event bus that manages subscriptions and dispatches events
pub struct EventBus {
    /// Subscriptions: event type -> set of component IDs
    subscriptions: HashMap<EventType, HashSet<ComponentId>>,
    /// Current event context (focus, areas, etc.)
    context: EventContext,
    /// Channel for sending actions
    action_tx: mpsc::UnboundedSender<Action>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            subscriptions: HashMap::new(),
            context: EventContext::default(),
            action_tx,
        }
    }

    /// Subscribe a component to an event type
    pub fn subscribe(&mut self, component: ComponentId, event_type: EventType) {
        self.subscriptions
            .entry(event_type)
            .or_default()
            .insert(component);
    }

    /// Subscribe a component to multiple event types
    pub fn subscribe_many(&mut self, component: ComponentId, event_types: &[EventType]) {
        for &event_type in event_types {
            self.subscribe(component, event_type);
        }
    }

    /// Unsubscribe a component from an event type
    pub fn unsubscribe(&mut self, component: ComponentId, event_type: EventType) {
        if let Some(subscribers) = self.subscriptions.get_mut(&event_type) {
            subscribers.remove(&component);
        }
    }

    /// Unsubscribe a component from all event types
    pub fn unsubscribe_all(&mut self, component: ComponentId) {
        for subscribers in self.subscriptions.values_mut() {
            subscribers.remove(&component);
        }
    }

    /// Get subscribers for an event type
    pub fn get_subscribers(&self, event_type: EventType) -> Vec<ComponentId> {
        self.subscriptions
            .get(&event_type)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all subscribers that should receive an event
    pub fn get_event_subscribers(&self, event: &Event) -> Vec<ComponentId> {
        let mut subscribers = HashSet::new();

        // If it's a global event, include Global subscribers
        if event.is_global() {
            if let Some(global_subs) = self.subscriptions.get(&EventType::Global) {
                subscribers.extend(global_subs.iter().copied());
            }
        }

        // Add type-specific subscribers
        if let Some(type_subs) = self.subscriptions.get(&event.event_type()) {
            subscribers.extend(type_subs.iter().copied());
        }

        subscribers.into_iter().collect()
    }

    /// Get mutable reference to context
    pub fn context_mut(&mut self) -> &mut EventContext {
        &mut self.context
    }

    /// Get reference to context
    pub fn context(&self) -> &EventContext {
        &self.context
    }

    /// Create an event with current context
    pub fn create_event(&self, kind: EventKind) -> Event {
        Event::new(kind, self.context.clone())
    }

    /// Get the action sender
    pub fn action_tx(&self) -> &mpsc::UnboundedSender<Action> {
        &self.action_tx
    }

    /// Update context from mouse position
    pub fn update_mouse_position(&mut self, x: u16, y: u16) {
        self.context.mouse_position = Some((x, y));
    }

    /// Update modifiers from key event
    pub fn update_modifiers(&mut self, modifiers: KeyModifiers) {
        self.context.modifiers = modifiers;
    }
}

/// Spawn the event polling task
pub fn spawn_event_poller(
    tx: mpsc::UnboundedSender<RawEvent>,
    poll_timeout: Duration,
    loop_sleep: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        const MAX_EVENTS_PER_BATCH: usize = 20;

        loop {
            tokio::time::sleep(loop_sleep).await;

            let mut events_processed = 0;
            while events_processed < MAX_EVENTS_PER_BATCH
                && event::poll(poll_timeout).unwrap_or(false)
            {
                events_processed += 1;
                if let Ok(evt) = event::read() {
                    let raw = match evt {
                        event::Event::Key(key) => Some(RawEvent::Key(key)),
                        event::Event::Mouse(mouse) => Some(RawEvent::Mouse(mouse)),
                        event::Event::Resize(w, h) => Some(RawEvent::Resize(w, h)),
                        _ => None,
                    };
                    if let Some(raw) = raw {
                        if tx.send(raw).is_err() {
                            debug!("Event channel closed, stopping poller");
                            return;
                        }
                    }
                }
            }
        }
    })
}

/// Process a raw event into an EventKind
pub fn process_raw_event(raw: RawEvent) -> EventKind {
    match raw {
        RawEvent::Key(key) => EventKind::Key(key),
        RawEvent::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollDown => EventKind::Scroll {
                column: mouse.column,
                row: mouse.row,
                delta: 1,
            },
            MouseEventKind::ScrollUp => EventKind::Scroll {
                column: mouse.column,
                row: mouse.row,
                delta: -1,
            },
            _ => EventKind::Mouse(mouse),
        },
        RawEvent::Resize(w, h) => EventKind::Resize(w, h),
    }
}
