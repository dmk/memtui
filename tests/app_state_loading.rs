mod support;

use rstest::rstest;

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
