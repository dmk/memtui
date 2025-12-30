mod support;

use std::time::{Duration, Instant};

use insta::assert_snapshot;
use rstest::rstest;

use memtui::action::CmdLineMode;
use memtui::backend::BackendCapabilities;

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
    app.app_state.cmdline_buffer = "user".to_string();
    app.app_state.search_results_local = search.indices;
    app.app_state.search_match_positions = search.match_positions;
    app.app_state.cmdline_mode = Some(memtui::action::CmdLineMode::Search);

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

#[rstest]
fn key_list_header_shows_total_count() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((60, 18)));
    assert!(
        rendered.contains("1 of 3"),
        "expected total count in key list header"
    );
}

#[rstest]
fn key_list_hides_type_column_when_disabled() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    freeze_animation(&mut app.ui_state);

    app.app_state.connection_manager.set_capabilities(
        "local",
        BackendCapabilities {
            supports_ttl: true,
            supports_scan: true,
            supports_raw_commands: true,
            supports_batch_get: true,
            supports_efficient_pattern_search: true,
            supports_type_display: false,
            supports_expiry_display: true,
            has_limited_key_listing: false,
        },
    );

    let rendered = buffer_to_string(&app.render((60, 18)));
    assert!(
        !rendered.contains("json"),
        "expected type column to be hidden for non-Redis backends"
    );
}

#[rstest]
fn key_list_shows_type_column_for_redis() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((60, 18)));
    assert!(
        rendered.contains("json"),
        "expected type column to be visible for Redis"
    );
}

#[rstest]
fn key_list_empty_database_shows_message() {
    let mut app = AppHarness::new().with_connection("local");
    app.app_state.keys.clear();
    app.app_state.total_key_count = Some(0);
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((60, 12)));
    assert!(
        rendered.contains("No keys"),
        "expected empty key list message"
    );
}

#[rstest]
fn status_bar_shows_error_message() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    app.app_state.error_message = Some("Scan failed".to_string());
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((60, 12)));
    assert!(
        rendered.contains("Scan failed"),
        "expected error message in status bar"
    );
}

#[rstest]
fn search_result_count_displayed_in_header() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());
    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_buffer = "user".to_string();
    app.app_state.search_results_local = vec![0, 1];
    app.app_state.search_selection_index = Some(0);
    freeze_animation(&mut app.ui_state);

    let rendered = buffer_to_string(&app.render((60, 12)));
    assert!(
        rendered.contains("2 found"),
        "expected search result count in key list header"
    );
}
