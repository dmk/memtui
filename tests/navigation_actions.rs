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

#[rstest]
fn selecting_new_key_clears_error() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Simulate an earlier load error
    app.app_state.error_message = Some("expired".to_string());
    app.app_state.selected_value = Some(memtui::types::Value {
        data: b"stale".to_vec(),
        value_type: memtui::types::ValueType::String,
        encoding: Some("utf8".to_string()),
    });

    // Select a different key; error should clear
    assert!(app.dispatch_action(Action::SelectKey(1)));

    assert_eq!(app.app_state.error_message, None);
    assert_eq!(app.app_state.selected_key_index, Some(1));
    assert!(app.app_state.selected_value.is_none());
}
