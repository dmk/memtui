//! Integration tests demonstrating tui-dispatch testing helpers.
//!
//! These tests showcase the fluent API, assertion macros, and other testing
//! utilities provided by tui-dispatch.

mod support;

use memtui::action::Action;
use rstest::rstest;
use tui_dispatch::{
    assert_emitted, assert_not_emitted, count_emitted, find_emitted, ActionAssertions,
};

use support::{fixtures, harness::AppHarness};

// ============================================================================
// Fluent Action Assertions
// ============================================================================

#[rstest]
fn action_assertions_count_and_empty() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Before any navigation, no actions should be emitted
    let actions = app.drain_actions();
    actions.assert_empty();

    // Navigate multiple times to emit actions
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);

    let actions = app.drain_actions();
    actions.assert_not_empty();
    actions.assert_count(3);
}

#[rstest]
fn action_assertions_all_match_and_none_match() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Navigate through the key list
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::PrevItem);

    let actions = app.drain_actions();

    // All emitted actions should be SelectKey variants
    actions.assert_all_match(|a| matches!(a, Action::SelectKey(_)));

    // None should be search-related
    actions.assert_none_match(|a| matches!(a, Action::StartSearch | Action::ClearSearch));
}

#[rstest]
fn action_assertions_first_and_any_matches() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Starting at key 0, navigate forward twice
    app.dispatch_action(Action::NextItem); // selects key 1
    app.dispatch_action(Action::NextItem); // selects key 2

    let actions = app.drain_actions();

    // First action should be SelectKey(1)
    actions.assert_first_matches(|a| matches!(a, Action::SelectKey(1)));

    // Should have SelectKey(2) somewhere
    actions.assert_any_matches(|a| matches!(a, Action::SelectKey(2)));
}

// ============================================================================
// Assertion Macros
// ============================================================================

#[rstest]
fn assert_emitted_macro_with_patterns() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);

    let actions = app.drain_actions();

    // Use assert_emitted! with exact pattern
    assert_emitted!(actions, Action::SelectKey(1));
    assert_emitted!(actions, Action::SelectKey(2));

    // Use with wildcard pattern
    assert_emitted!(actions, Action::SelectKey(_));
}

#[rstest]
fn assert_not_emitted_macro() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Only do navigation
    app.dispatch_action(Action::NextItem);

    let actions = app.drain_actions();

    // Navigation shouldn't emit search-related actions
    assert_not_emitted!(actions, Action::StartSearch);
    assert_not_emitted!(actions, Action::ClearSearch);
    assert_not_emitted!(actions, Action::ConfirmSearch);

    // Should not emit SelectKey(99) since we only have 3 keys
    assert_not_emitted!(actions, Action::SelectKey(99));
}

#[rstest]
fn count_emitted_macro() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Navigate 5 times (wrapping with 3 keys: 0->1->2->0->1->2)
    for _ in 0..5 {
        app.dispatch_action(Action::NextItem);
    }

    let actions = app.drain_actions();

    // Should have 5 SelectKey actions total
    let select_key_count = count_emitted!(actions, Action::SelectKey(_));
    assert_eq!(select_key_count, 5);

    // Count specific indices (with wrapping: 1, 2, 0, 1, 2)
    let select_key_1_count = count_emitted!(actions, Action::SelectKey(1));
    assert_eq!(select_key_1_count, 2, "SelectKey(1) should appear twice");

    let select_key_2_count = count_emitted!(actions, Action::SelectKey(2));
    assert_eq!(select_key_2_count, 2, "SelectKey(2) should appear twice");

    let select_key_0_count = count_emitted!(actions, Action::SelectKey(0));
    assert_eq!(
        select_key_0_count, 1,
        "SelectKey(0) should appear once (wrap)"
    );
}

#[rstest]
fn find_emitted_macro() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);

    let actions = app.drain_actions();

    // Find a SelectKey action and extract its value
    let found = find_emitted!(actions, Action::SelectKey(_));
    assert!(found.is_some(), "Should find at least one SelectKey");

    if let Some(Action::SelectKey(idx)) = found {
        assert!(*idx <= 2, "Index should be within key bounds");
    }
}

// ============================================================================
// Combined Patterns
// ============================================================================

#[rstest]
fn dispatch_sequence_with_assertions() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Use dispatch_sequence helper
    let rendered = app.dispatch_sequence([Action::NextItem, Action::NextItem, Action::PrevItem]);

    assert!(rendered, "Navigation should trigger render");

    let actions = app.drain_actions();

    // Verify sequence using multiple assertion styles
    actions.assert_count(3);
    actions.assert_first_matches(|a| matches!(a, Action::SelectKey(1)));
    assert_emitted!(actions, Action::SelectKey(2));

    // Final selection should be back at 1 (0 -> 1 -> 2 -> 1)
    assert_eq!(app.ui_state.key_list.state.selected(), Some(1));
}

#[rstest]
fn search_mode_action_flow() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Navigate to key 2 first
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::NextItem);
    assert_eq!(app.ui_state.key_list.state.selected(), Some(2));
    app.drain_actions(); // Clear navigation actions

    // Start search mode
    app.dispatch_action(Action::StartSearch);
    assert!(app.app_state.is_searching);

    // Clear search (exits search mode and resets to first key)
    app.dispatch_action(Action::ClearSearch);

    // Verify search mode is exited and selection reset
    assert!(!app.app_state.is_searching);
    assert!(app.app_state.search_query.is_empty());
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));
    assert_eq!(app.app_state.selected_key_index, Some(0));
}

#[rstest]
fn no_actions_for_noop_operations() {
    let mut app = AppHarness::new();

    // Operations on empty state shouldn't emit actions
    app.dispatch_action(Action::NextItem);
    app.dispatch_action(Action::PrevItem);

    let actions = app.drain_actions();

    // Without keys loaded, navigation is a no-op
    actions.assert_empty();
}
