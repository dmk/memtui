mod support;

use memtui::action::Action;
use rstest::rstest;

use support::{fixtures, harness::AppHarness};

#[rstest]
fn key_navigation_wraps_and_emits_select() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Initial selection is set to 0 by the fixture helper.
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));

    assert!(app.dispatch_action(Action::NextItem));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(1));

    assert!(app.dispatch_action(Action::NextItem));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(2));

    // Wrap to start
    assert!(app.dispatch_action(Action::NextItem));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));

    // Wrap to end
    assert!(app.dispatch_action(Action::PrevItem));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(2));

    // Ensure emitted actions track the selection changes.
    let emitted = app.drain_actions();
    assert!(
        emitted.iter().any(|a| matches!(a, Action::SelectKey(2))),
        "navigation should emit SelectKey actions for current index"
    );
}
