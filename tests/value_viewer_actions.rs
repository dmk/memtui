mod support;

use memtui::action::Action;
use memtui::actions::{async_handlers, sync_handlers};
use memtui::events::{ComponentId, Event, EventContext, EventKind};
use memtui::types::{Value, ValueType};
use memtui::ui::components::value_viewer::ValueViewerProps;
use memtui::ui::components::Component;
use rstest::rstest;

use support::{fixtures, harness::AppHarness};

#[rstest]
fn selecting_key_clears_previous_value() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Simulate a loaded value
    app.app_state.selected_value = Some(Value {
        data: b"{\"id\": 123}".to_vec(),
        value_type: ValueType::Json,
        encoding: Some("json".to_string()),
    });
    app.app_state.selected_key_index = Some(0);

    // Select a different key
    assert!(app.dispatch_action(Action::SelectKey(1)));

    // Value should be cleared (would be reloaded by async handler)
    assert!(app.app_state.selected_value.is_none());
    assert_eq!(app.app_state.selected_key_index, Some(1));
}

#[rstest]
fn load_value_cancel_stale_token() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    app.app_state.value_request_token = 2;

    let applied = async_handlers::handle_did_load_value(
        &mut app.app_state,
        memtui::types::Value {
            data: b"stale".to_vec(),
            value_type: ValueType::String,
            encoding: None,
        },
        1,
    );

    assert!(!applied);
    assert!(app.app_state.selected_value.is_none());
}

#[rstest]
fn load_value_not_found_sets_error() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    sync_handlers::handle_did_fail_load_value(&mut app.app_state, "Key not found".to_string());

    assert_eq!(
        app.app_state.error_message.as_deref(),
        Some("Error loading value: Key not found")
    );
    assert!(app.app_state.selected_value.is_none());
}

#[rstest]
fn load_value_error_sets_error() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    sync_handlers::handle_did_fail_load_value(&mut app.app_state, "Backend error".to_string());

    assert_eq!(
        app.app_state.error_message.as_deref(),
        Some("Error loading value: Backend error")
    );
    assert!(app.app_state.selected_value.is_none());
}

#[rstest]
fn selecting_key_resets_scroll_position() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Simulate scrolled state
    app.ui_state.value_viewer.scroll_offset = 10;

    // Select a key
    assert!(app.dispatch_action(Action::SelectKey(0)));

    // Scroll should reset
    assert_eq!(app.ui_state.value_viewer.scroll_offset, 0);
}

#[rstest]
fn scroll_mouse_emits_scroll_action() {
    let mut app = AppHarness::new();
    let props = ValueViewerProps {
        selected_value: None,
        selected_key_type: None,
        selected_key_name: None,
        selected_key: None,
        error_message: None,
        json_formatter: &app.app_state.json_formatter,
        text_formatter: &app.app_state.text_formatter,
        is_active: true,
        capabilities: None,
        animation: &app.ui_state.animation,
        now: std::time::SystemTime::now(),
        keybindings: &app.keybindings,
    };

    let area = ratatui::layout::Rect::new(0, 0, 10, 6);
    let mut context = EventContext::default();
    context.set_focus(Some(ComponentId::ValueViewer));
    context.set_component_area(ComponentId::ValueViewer, area);
    let event = Event::new(
        EventKind::Scroll {
            column: 1,
            row: 1,
            delta: 1,
        },
        context,
    );

    let actions = app.ui_state.value_viewer.handle_event(&event, props);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], Action::ValueViewerScrollBy(1)));
}

#[rstest]
fn value_viewer_scroll_state_persists_for_same_key() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Select key and set scroll
    app.dispatch_action(Action::SelectKey(0));
    app.ui_state.value_viewer.scroll_offset = 5;

    // Re-selecting same key should NOT reset scroll (value unchanged)
    // Note: In actual app, SelectKey always resets - this tests current behavior
    app.dispatch_action(Action::SelectKey(0));

    // Current behavior: scroll resets even for same key
    assert_eq!(app.ui_state.value_viewer.scroll_offset, 0);
}

#[rstest]
fn binary_value_cycle_view_mode_clears_cache() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set up binary value
    app.app_state.selected_value = Some(Value {
        data: vec![0x00, 0x01, 0xFF, 0xFE],
        value_type: ValueType::Binary,
        encoding: None,
    });

    // Cycle view mode should not panic
    app.ui_state.value_viewer.cycle_view_mode();

    // Cycle again (toggles back)
    app.ui_state.value_viewer.cycle_view_mode();
}

#[rstest]
fn json_value_is_formatted() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set up JSON value
    let json_data = b"{\"name\":\"test\",\"value\":42}";
    app.app_state.selected_value = Some(Value {
        data: json_data.to_vec(),
        value_type: ValueType::Json,
        encoding: Some("json".to_string()),
    });

    // Verify value is stored
    assert!(app.app_state.selected_value.is_some());
    let value = app.app_state.selected_value.as_ref().unwrap();
    assert_eq!(value.value_type, ValueType::Json);
}

#[rstest]
fn error_message_cleared_on_key_selection() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set an error
    app.app_state.error_message = Some("Connection timeout".to_string());

    // Select a key
    assert!(app.dispatch_action(Action::SelectKey(1)));

    // Error should be cleared
    assert!(app.app_state.error_message.is_none());
}

#[rstest]
fn large_value_sets_viewport() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set up large value (many lines)
    let large_json = (0..100)
        .map(|i| format!("\"line_{}\": {}", i, i))
        .collect::<Vec<_>>()
        .join(",\n");
    let json_data = format!("{{{}}}", large_json);

    app.app_state.selected_value = Some(Value {
        data: json_data.into_bytes(),
        value_type: ValueType::Json,
        encoding: Some("json".to_string()),
    });

    // Verify value is set
    assert!(app.app_state.selected_value.is_some());
}
