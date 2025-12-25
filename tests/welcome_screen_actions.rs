mod support;

use memtui::action::Action;
use rstest::rstest;

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
