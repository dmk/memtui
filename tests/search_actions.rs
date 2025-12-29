mod support;

use memtui::action::{Action, CmdLineMode};
use memtui::actions::async_handlers;
use memtui::search::fuzzy_search_keys_with_positions;
use rstest::rstest;
use tui_dispatch::{assert_emitted, ActionAssertions};

use support::{fixtures, harness::AppHarness};

#[rstest]
fn search_results_update_selection_and_emit_action() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert!(app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search)));
    let token = app.app_state.search_token;
    let query = "abc";
    app.app_state.cmdline_buffer = query.to_string();

    let search = fuzzy_search_keys_with_positions(&app.app_state.keys, query);
    assert_eq!(search.indices, vec![2]);

    let applied = async_handlers::handle_did_search_local(
        &mut app.app_state,
        &app.action_tx,
        search.indices.clone(),
        search.match_positions.clone(),
        token,
    );
    assert!(applied, "search handler should accept current token");
    assert_eq!(app.app_state.search_results_local, vec![2]);
    assert_eq!(app.app_state.search_selection_index, Some(0));
    assert!(app.app_state.search_match_positions.contains_key(&2));

    // Use fluent API for action assertions (predicate-based since Action doesn't impl PartialEq)
    let actions = app.drain_actions();
    actions.assert_not_empty();
    actions.assert_first_matches(|a| matches!(a, Action::SelectKey(2)));
    // Also works with assert_emitted! macro
    assert_emitted!(actions, Action::SelectKey(2));

    for action in actions {
        app.dispatch_action(action);
    }

    assert_eq!(app.app_state.selected_key_index, Some(2));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(2));
}

#[rstest]
fn clear_search_resets_results_and_focuses_first_key() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert!(app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search)));
    let token = app.app_state.search_token;
    let query = "abc";
    app.app_state.cmdline_buffer = query.to_string();

    let search = fuzzy_search_keys_with_positions(&app.app_state.keys, query);
    assert_eq!(search.indices, vec![2]);

    async_handlers::handle_did_search_local(
        &mut app.app_state,
        &app.action_tx,
        search.indices.clone(),
        search.match_positions.clone(),
        token,
    );
    for action in app.drain_actions() {
        app.dispatch_action(action);
    }
    assert_eq!(app.app_state.selected_key_index, Some(2));
    assert_eq!(app.ui_state.key_list.state.selected(), Some(2));

    assert!(app.dispatch_action(Action::CmdLineClear));

    assert!(app.app_state.is_searching());
    assert!(app.app_state.search_results_local.is_empty());
    assert!(app.app_state.search_match_positions.is_empty());
    assert_eq!(app.app_state.cmdline_buffer, "");
    assert_eq!(app.ui_state.key_list.state.selected(), Some(0));
    assert_eq!(app.app_state.selected_key_index, Some(0));
}
