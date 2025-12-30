mod support;

use memtui::action::Action;
use memtui::actions::async_handlers;
use memtui::actions::sync_handlers;
use rstest::rstest;
use tui_dispatch::assert_emitted;

use support::{fixtures, harness::AppHarness};

#[rstest]
fn apply_scan_result_populates_keys_and_selection() {
    let mut app = AppHarness::new().with_connection("local");
    let keys = fixtures::sample_keys();

    app.apply_scan_result(keys.clone(), true, false);

    assert_eq!(app.app_state.keys.len(), keys.len());
    assert!(app.app_state.keys.iter().all(|k| k.is_some()));
    assert_eq!(app.app_state.total_key_count, Some(keys.len() as u64));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));
    assert!(!app.app_state.is_loading_keys);
}

#[rstest]
fn initial_scan_emits_select_action() {
    let mut app = AppHarness::new().with_connection("local");
    let keys = fixtures::sample_keys();

    async_handlers::handle_did_scan_keys(
        &mut app.app_state,
        &mut app.ui_state,
        &app.action_tx,
        keys,
        None,
        false,
        Some(3),
        true,
        None,
    );

    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));

    let emitted = app.drain_actions();
    assert_emitted!(emitted, Action::SelectKey(0));
}

#[rstest]
fn load_keys_error_sets_status_message() {
    let mut app = AppHarness::new().with_connection("local");
    app.app_state.is_loading_keys = true;

    sync_handlers::handle_did_fail_scan_keys(&mut app.app_state, "Scan failed".to_string());

    assert_eq!(app.app_state.error_message.as_deref(), Some("Scan failed"));
    assert!(!app.app_state.is_loading_keys);
}
