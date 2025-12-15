mod support;

use std::time::{Duration, Instant};

use insta::assert_snapshot;
use rstest::rstest;

use support::{
    fixtures,
    harness::AppHarness,
    render::{buffer_to_string, fixed_now},
};

fn freeze_animation(ui_state: &mut memtui::ui::UiState) {
    ui_state.animation.start_time = Instant::now();
}

#[rstest]
fn key_list_renders_with_and_without_search() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    freeze_animation(&mut app.ui_state);

    let normal = buffer_to_string(&app.render((60, 18)));
    assert_snapshot!("key_list_normal", normal);

    // Populate search state manually for deterministic highlighting
    let search = memtui::search::fuzzy_search_keys_with_positions(&app.app_state.keys, "user");
    app.app_state.search_query = "user".to_string();
    app.app_state.search_results_local = search.indices;
    app.app_state.search_match_positions = search.match_positions;
    app.app_state.is_searching = false; // query is active, not editing

    let with_search = buffer_to_string(&app.render((60, 18)));
    assert_snapshot!("key_list_search_user", with_search);
}

#[rstest]
fn key_list_renders_ttl_alignment_and_urgency() {
    let now = fixed_now();
    let keys = vec![
        memtui::types::KeyMetadata {
            name: "ttl:hours".to_string(),
            value_type: memtui::types::ValueType::Hash,
            size_bytes: 0,
            ttl: None,
            last_accessed: None,
            encoding: None,
            expires_at: Some(now + Duration::from_secs(7200)), // normal
        },
        memtui::types::KeyMetadata {
            name: "ttl:minutes".to_string(),
            value_type: memtui::types::ValueType::List,
            size_bytes: 0,
            ttl: None,
            last_accessed: None,
            encoding: None,
            expires_at: Some(now + Duration::from_secs(240)), // soon
        },
        memtui::types::KeyMetadata {
            name: "ttl:seconds".to_string(),
            value_type: memtui::types::ValueType::String,
            size_bytes: 0,
            ttl: None,
            last_accessed: None,
            encoding: None,
            expires_at: Some(now + Duration::from_secs(30)), // imminent
        },
    ];

    let mut app = AppHarness::new().with_connection("local").with_keys(keys);
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((70, 12)));
    assert_snapshot!("key_list_ttl_styles", rendered);
}
