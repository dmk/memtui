mod support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use memtui::action::Action;
use memtui::events::{ComponentId, Event, EventContext, EventKind};
use memtui::types::ConnectionConfig;
use memtui::ui::components::connection_list::ConnectionListProps;
use memtui::ui::components::Component;
use rstest::rstest;
use tui_dispatch::ActionAssertions;

use support::{fixtures, harness::AppHarness};

fn alpha_and_beta() -> (ConnectionConfig, ConnectionConfig) {
    let mut alpha = fixtures::connection("alpha");
    alpha.name = "Alpha".to_string();
    let mut beta = fixtures::connection("beta");
    beta.name = "Beta".to_string();
    (alpha, beta)
}

#[rstest]
fn open_palette_selects_active_connection() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new().with_connection_config(beta.clone());
    app = app.with_connection_config(alpha);
    app.app_state.connection_manager.set_active(&beta.id);

    // Start from a different selection to ensure OpenConnectionPalette recalculates.
    app.ui_state.connection_list.state.select(Some(0));

    assert!(app.dispatch_action(Action::OpenConnectionPalette));
    assert!(app.ui_state.show_connection_palette);
    assert_eq!(
        app.ui_state.connection_list.state.selected(),
        Some(1),
        "active connection should be highlighted based on sorted configs"
    );
}

#[rstest]
fn open_palette_without_connections_clears_selection() {
    let mut app = AppHarness::new();

    assert!(app.dispatch_action(Action::OpenConnectionPalette));
    assert!(app.ui_state.show_connection_palette);
    assert_eq!(
        app.ui_state.connection_list.state.selected(),
        None,
        "palette should have no selection when there are no connections"
    );
}

#[rstest]
fn delete_connection_removes_from_list() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new()
        .with_connection_config(alpha.clone())
        .with_connection_config(beta.clone());

    // Open palette and select alpha
    app.dispatch_action(Action::OpenConnectionPalette);
    app.ui_state.connection_list.state.select(Some(0));

    // Verify we have 2 connections
    assert_eq!(app.app_state.connection_manager.get_configs().len(), 2);

    // Delete alpha
    assert!(app.dispatch_action(Action::DeleteConnection(alpha.id.clone())));

    // Verify only beta remains
    let configs = app.app_state.connection_manager.get_configs();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].id, beta.id);
}

#[rstest]
fn delete_last_connection_closes_palette() {
    let alpha = fixtures::connection("alpha");

    let mut app = AppHarness::new().with_connection_config(alpha.clone());

    app.dispatch_action(Action::OpenConnectionPalette);
    assert!(app.ui_state.show_connection_palette);

    // Delete the only connection
    assert!(app.dispatch_action(Action::DeleteConnection(alpha.id.clone())));

    // Palette should close when no connections remain
    assert!(!app.ui_state.show_connection_palette);
    assert_eq!(app.app_state.connection_manager.get_configs().len(), 0);
}

#[rstest]
fn connection_list_scroll_navigates() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new()
        .with_connection_config(alpha)
        .with_connection_config(beta);

    app.dispatch_action(Action::OpenConnectionPalette);
    app.ui_state.connection_list.state.select(Some(0));

    // Scroll down (ConnectionListNext)
    assert!(app.dispatch_action(Action::ConnectionListNext));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(1));

    // Scroll up (ConnectionListPrev)
    assert!(app.dispatch_action(Action::ConnectionListPrev));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(0));

    // Scroll up wraps to end
    assert!(app.dispatch_action(Action::ConnectionListPrev));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(1));
}

#[rstest]
fn palette_navigation_uses_next_prev_item() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new()
        .with_connection_config(alpha)
        .with_connection_config(beta);

    app.dispatch_action(Action::OpenConnectionPalette);
    app.ui_state.connection_list.state.select(Some(0));

    assert!(app.dispatch_action(Action::NextItem));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(1));

    assert!(app.dispatch_action(Action::PrevItem));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(0));

    assert!(app.dispatch_action(Action::PrevItem));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(1));
}

#[rstest]
fn palette_enter_focuses_selected_connection() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new()
        .with_connection_config(alpha.clone())
        .with_connection_config(beta);

    app.dispatch_action(Action::OpenConnectionPalette);
    app.ui_state.connection_list.state.select(Some(0));

    assert!(app.dispatch_action(Action::Enter));
    assert!(!app.ui_state.show_connection_palette);

    let actions = app.drain_actions();
    actions.assert_any_matches(|a| matches!(a, Action::FocusConnection(id) if id == &alpha.id));
}

#[rstest]
fn palette_delete_connection_via_keybinding() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new()
        .with_connection_config(alpha.clone())
        .with_connection_config(beta);

    let props = ConnectionListProps {
        configs: app.app_state.connection_manager.get_configs(),
        active_id: app.app_state.connection_manager.get_active_id(),
        statuses: app.app_state.connection_manager.get_statuses(),
        is_active: true,
        animation: &app.ui_state.animation,
        keybindings: &app.keybindings,
    };

    let mut context = EventContext::default();
    context.set_focus(Some(ComponentId::ConnectionPalette));
    let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    let event = Event::new(EventKind::Key(key), context);

    let actions = app.ui_state.connection_list.handle_event(&event, props);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        Action::DeleteConnection(id) if id == &alpha.id
    ));
}
