mod support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use memtui::action::Action;
use memtui::events::{ComponentId, Event, EventContext, EventKind};
use memtui::ui::components::Component;
use rstest::rstest;

use support::render::buffer_to_string;
use support::{fixtures, harness::AppHarness};

fn setup_with_recent_connections() -> AppHarness {
    let alpha = fixtures::connection("alpha");
    let beta = fixtures::connection("beta");
    let gamma = fixtures::connection("gamma");

    let mut app = AppHarness::new()
        .with_connection_config(alpha.clone())
        .with_connection_config(beta.clone())
        .with_connection_config(gamma.clone());

    // Set up recent connection IDs (simulates previous usage)
    app.ui_state.recent_connection_ids = vec![alpha.id, beta.id, gamma.id];

    // Ensure no active connection (shows welcome screen)
    // Note: with_connection_config sets active, so we need to clear it
    // The welcome screen is shown when there's no active connection
    app.ui_state.welcome_screen.state.select(Some(0));

    app
}

#[rstest]
fn welcome_scroll_navigates_recent_connections() {
    let mut app = setup_with_recent_connections();

    // Start at first item
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(0));

    // Scroll down
    assert!(app.dispatch_action(Action::WelcomeNextItem));
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(1));

    // Scroll down again
    assert!(app.dispatch_action(Action::WelcomeNextItem));
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(2));

    // Scroll up
    assert!(app.dispatch_action(Action::WelcomePrevItem));
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(1));
}

#[rstest]
fn welcome_scroll_wraps_around() {
    let mut app = setup_with_recent_connections();

    // Start at first item
    app.ui_state.welcome_screen.state.select(Some(0));

    // Scroll up from first should wrap to last
    assert!(app.dispatch_action(Action::WelcomePrevItem));
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(2));

    // Scroll down from last should wrap to first
    assert!(app.dispatch_action(Action::WelcomeNextItem));
    assert_eq!(app.ui_state.welcome_screen.state.selected(), Some(0));
}

#[rstest]
fn welcome_shows_on_start_without_connections() {
    let mut app = AppHarness::new();

    let rendered = buffer_to_string(&app.render((80, 20)));
    assert!(
        rendered.contains("No recent connections"),
        "expected welcome empty state message"
    );
}

#[rstest]
fn welcome_recent_connections_rendered() {
    let alpha = fixtures::connection("alpha");

    let mut app = AppHarness::new();
    app.app_state
        .connection_manager
        .add_connection(alpha.clone());
    app.ui_state.recent_connection_ids = vec![alpha.id.clone()];

    let rendered = buffer_to_string(&app.render((80, 20)));
    assert!(rendered.contains("Recent Connections"));
    assert!(rendered.to_lowercase().contains("alpha"));
}

#[rstest]
fn welcome_select_emits_focus_connection() {
    let alpha = fixtures::connection("alpha");

    let mut app = AppHarness::new();
    app.app_state
        .connection_manager
        .add_connection(alpha.clone());
    app.ui_state.recent_connection_ids = vec![alpha.id.clone()];

    let configs = app.app_state.connection_manager.get_configs();
    let recent_configs = app
        .ui_state
        .recent_connection_ids
        .iter()
        .filter_map(|id| configs.iter().find(|c| c.id == *id).copied())
        .collect();

    let props = memtui::ui::components::welcome::WelcomeScreenProps {
        recent_configs,
        animation: &app.ui_state.animation,
        keybindings: &app.keybindings,
    };

    let mut context = EventContext::default();
    context.set_focus(Some(ComponentId::WelcomeScreen));
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let event = Event::new(EventKind::Key(key), context);

    let actions = app.ui_state.welcome_screen.handle_event(&event, props);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        Action::FocusConnection(id) if id == &alpha.id
    ));
}
