mod support;

use memtui::action::{Action, CmdLineMode};
use memtui::ui::Panel;
use rstest::rstest;
use tui_dispatch::assert_emitted;

use support::{fixtures, harness::AppHarness};

#[rstest]
fn next_panel_cycles_through_panels() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert_eq!(app.ui_state.active_panel, Panel::Keys);

    app.dispatch_action(Action::NextPanel);
    assert_eq!(app.ui_state.active_panel, Panel::Value);

    app.dispatch_action(Action::NextPanel);
    assert_eq!(app.ui_state.active_panel, Panel::Keys);
}

#[rstest]
fn prev_panel_cycles_backwards() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert_eq!(app.ui_state.active_panel, Panel::Keys);

    app.dispatch_action(Action::PrevPanel);
    assert_eq!(app.ui_state.active_panel, Panel::Value);

    app.dispatch_action(Action::PrevPanel);
    assert_eq!(app.ui_state.active_panel, Panel::Keys);
}

#[rstest]
fn focus_panel_directly_sets_panel() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    app.dispatch_action(Action::FocusPanel(Panel::Value));
    assert_eq!(app.ui_state.active_panel, Panel::Value);

    app.dispatch_action(Action::FocusPanel(Panel::Keys));
    assert_eq!(app.ui_state.active_panel, Panel::Keys);
}

#[rstest]
fn next_item_in_keys_panel_navigates_keys() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert_eq!(app.ui_state.active_panel, Panel::Keys);
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));

    app.dispatch_action(Action::NextItem);

    // Drain the SelectKey action that gets queued
    let actions = app.drain_actions();
    assert_emitted!(actions, Action::SelectKey(1));
}

#[rstest]
fn prev_item_in_keys_panel_navigates_keys() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Start at index 1
    app.dispatch_action(Action::SelectKey(1));

    app.dispatch_action(Action::PrevItem);

    let actions = app.drain_actions();
    assert_emitted!(actions, Action::SelectKey(0));
}

#[rstest]
fn switching_panel_exits_search_mode() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Enter search mode
    app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search));
    assert!(app.app_state.is_searching());

    // Switch panels
    app.dispatch_action(Action::NextPanel);

    // Should deactivate cmdline but keep search visible
    assert!(!app.app_state.cmdline_active);
    assert!(app.app_state.is_searching());
}

#[rstest]
fn focus_panel_exits_search_mode() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Enter search mode
    app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search));
    assert!(app.app_state.is_searching());

    // Focus another panel directly
    app.dispatch_action(Action::FocusPanel(Panel::Value));

    // Should deactivate cmdline but keep search visible
    assert!(!app.app_state.cmdline_active);
    assert!(app.app_state.is_searching());
}

#[rstest]
fn pane_ratio_clamped_to_bounds() {
    let mut app = AppHarness::new();

    // Get bounds
    let min = app.ui_state.pane_split.min;
    let max = app.ui_state.pane_split.max;

    // Try to set below minimum
    app.dispatch_action(Action::SetPaneRatio(0.01));
    assert!(app.ui_state.pane_split.ratio >= min);

    // Try to set above maximum
    app.dispatch_action(Action::SetPaneRatio(0.99));
    assert!(app.ui_state.pane_split.ratio <= max);

    // Set in valid range
    app.dispatch_action(Action::SetPaneRatio(0.5));
    assert!((app.ui_state.pane_split.ratio - 0.5).abs() < 0.01);
}

#[rstest]
fn resize_mode_toggle() {
    let mut app = AppHarness::new();

    assert!(!app.ui_state.is_resizing);

    app.dispatch_action(Action::StartResize);
    assert!(app.ui_state.is_resizing);

    app.dispatch_action(Action::EndResize);
    assert!(!app.ui_state.is_resizing);
}

#[rstest]
fn combined_navigation_sequence() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Complex sequence
    let rendered = app.dispatch_many([
        Action::FocusPanel(Panel::Value),
        Action::SetPaneRatio(0.4),
        Action::FocusPanel(Panel::Keys),
        Action::NextItem,
        Action::NextPanel,
    ]);

    assert!(rendered);
    assert_eq!(app.ui_state.active_panel, Panel::Value);
    assert!((app.ui_state.pane_split.ratio - 0.4).abs() < 0.01);
}

#[rstest]
fn key_selection_clears_value_and_error() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set up state
    app.app_state.selected_value = Some(memtui::types::Value {
        data: b"test".to_vec(),
        value_type: memtui::types::ValueType::String,
        encoding: None,
    });
    app.app_state.error_message = Some("test error".to_string());

    // Select a different key
    app.dispatch_action(Action::SelectKey(2));

    // Value and error should be cleared
    assert!(app.app_state.selected_value.is_none());
    assert!(app.app_state.error_message.is_none());
    assert_eq!(app.app_state.selected_key_index, Some(2));
}
