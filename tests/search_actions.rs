mod support;

use memtui::action::{Action, CmdLineMode};
use memtui::actions::async_handlers;
use memtui::search::fuzzy_search_keys_with_positions;
use memtui::types::{KeyMetadata, KeyScanResult, ValueType};
use rstest::rstest;
use tui_dispatch::{assert_emitted, ActionAssertions};

use memtui::backend::{Backend, MockBackend};
use std::sync::Arc;
use support::{fixtures, harness::AppHarness, render::buffer_to_string};
use tokio::sync::RwLock;
use tokio::time::{advance, Duration};

fn make_key(name: &str) -> Option<KeyMetadata> {
    Some(KeyMetadata {
        name: name.to_string(),
        value_type: ValueType::String,
        size_bytes: 0,
        ttl: None,
        last_accessed: None,
        encoding: None,
        expires_at: None,
    })
}

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

#[rstest]
fn search_local_no_results_clears_selection() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert!(app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search)));
    let token = app.app_state.search_token;
    app.app_state.cmdline_buffer = "zzz".to_string();

    let search = fuzzy_search_keys_with_positions(&app.app_state.keys, "zzz");
    assert!(search.indices.is_empty());

    let applied = async_handlers::handle_did_search_local(
        &mut app.app_state,
        &app.action_tx,
        search.indices,
        search.match_positions,
        token,
    );
    assert!(applied);
    assert!(app.app_state.search_results_local.is_empty());
    assert!(app.app_state.search_match_positions.is_empty());
    assert_eq!(app.app_state.search_selection_index, None);
    assert!(app.drain_actions().is_empty());

    let rendered = buffer_to_string(&app.render((60, 12)));
    assert!(
        rendered.contains("No matches found"),
        "expected empty search state to render a hint"
    );
}

#[rstest]
fn search_local_ranking_prefers_exact_match() {
    let keys = vec![make_key("user"), make_key("user:123"), make_key("usr")];
    let result = fuzzy_search_keys_with_positions(&keys, "user");
    assert_eq!(result.indices.first().copied(), Some(0));
}

#[rstest]
fn search_local_highlight_positions_recorded() {
    let keys = vec![make_key("user:123"), make_key("session:abc")];
    let result = fuzzy_search_keys_with_positions(&keys, "u12");

    assert!(result.indices.contains(&0));
    assert!(result
        .match_positions
        .get(&0)
        .map(|positions| !positions.is_empty())
        .unwrap_or(false));
}

#[ignore] // TODO: async timing issues with start_paused
#[tokio::test(start_paused = true)]
async fn search_server_redis_returns_matches() {
    let mut app = AppHarness::new();
    let config = fixtures::connection("local");
    app.app_state
        .connection_manager
        .add_connection(config.clone());

    let mut backend = MockBackend::new(false);
    backend.connect().await.expect("mock backend connect");
    let caps = backend.capabilities();
    let backend: Box<dyn Backend> = Box::new(backend);
    let backend_arc = Arc::new(RwLock::new(backend));
    app.app_state
        .connection_manager
        .register_connection(&config.id, backend_arc, caps);

    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_active = true;
    app.app_state.cmdline_buffer = "user".to_string();

    async_handlers::trigger_search(&mut app.app_state, &app.action_tx);
    app.drain_actions();

    advance(Duration::from_millis(200)).await;
    tokio::task::yield_now().await;

    let actions = app.drain_actions();
    let mut found = false;
    for action in actions {
        if let Action::DidSearchServer { result, .. } = action {
            assert!(result.keys.iter().any(|k| k.name.contains("user")));
            found = true;
        }
    }
    assert!(found, "expected server search results");
}

#[ignore] // TODO: async timing issues with start_paused
#[tokio::test(start_paused = true)]
async fn search_server_debounce_delays_results() {
    let mut app = AppHarness::new();
    let config = fixtures::connection("local");
    app.app_state
        .connection_manager
        .add_connection(config.clone());

    let mut backend = MockBackend::new(false);
    backend.connect().await.expect("mock backend connect");
    let caps = backend.capabilities();
    let backend: Box<dyn Backend> = Box::new(backend);
    let backend_arc = Arc::new(RwLock::new(backend));
    app.app_state
        .connection_manager
        .register_connection(&config.id, backend_arc, caps);

    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_active = true;
    app.app_state.cmdline_buffer = "user".to_string();

    async_handlers::trigger_search(&mut app.app_state, &app.action_tx);

    let early = app.drain_actions();
    assert!(
        !early
            .iter()
            .any(|a| matches!(a, Action::DidSearchServer { .. })),
        "server search should be debounced"
    );

    advance(Duration::from_millis(200)).await;
    tokio::task::yield_now().await;

    let actions = app.drain_actions();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::DidSearchServer { .. })),
        "expected server search after debounce"
    );
}

#[tokio::test]
async fn search_server_loading_flag_set() {
    let mut app = AppHarness::new();
    let config = fixtures::connection("local");
    app.app_state
        .connection_manager
        .add_connection(config.clone());

    let backend = MockBackend::new(false);
    let caps = backend.capabilities();
    let backend: Box<dyn Backend> = Box::new(backend);
    let backend_arc = Arc::new(RwLock::new(backend));
    app.app_state
        .connection_manager
        .register_connection(&config.id, backend_arc, caps);

    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_active = true;
    app.app_state.cmdline_buffer = "user".to_string();

    async_handlers::trigger_search(&mut app.app_state, &app.action_tx);

    assert!(app.app_state.is_server_searching);
}

#[rstest]
fn search_server_fallback_keeps_local_results() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_active = true;
    app.app_state.cmdline_buffer = "user".to_string();
    app.app_state.search_results_local = vec![0, 1];
    app.app_state.search_selection_index = Some(0);
    app.app_state.search_token = 1;
    app.app_state.is_server_searching = true;

    let applied =
        async_handlers::handle_did_search_server(&mut app.app_state, &app.action_tx, vec![], 1);

    assert!(applied);
    assert!(!app.app_state.is_server_searching);
    assert_eq!(app.app_state.search_results_local, vec![0, 1]);
}

#[rstest]
fn search_server_merge_dedupes_existing_keys() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert!(app.dispatch_action(Action::SetCmdLineMode(CmdLineMode::Search)));
    app.app_state.cmdline_buffer = "user".to_string();
    app.app_state.is_server_searching = true;

    let local = fuzzy_search_keys_with_positions(&app.app_state.keys, "user");
    app.app_state.search_results_local = local.indices;
    app.app_state.search_match_positions = local.match_positions;
    app.app_state.search_selection_index = Some(0);

    let token = app.app_state.search_token;
    let original_len = app.app_state.keys.len();

    let duplicate = fixtures::sample_keys()[0].clone();
    let mut new_key = duplicate.clone();
    new_key.name = "user:999".to_string();

    let result = KeyScanResult {
        keys: vec![duplicate, new_key.clone()],
        cursor: None,
        has_more: false,
    };

    app.apply_server_search(result, token);

    assert!(!app.app_state.is_server_searching);
    assert_eq!(app.app_state.keys.len(), original_len + 1);
    assert_eq!(
        app.app_state.keys[original_len].as_ref().unwrap().name,
        "user:999"
    );
    assert!(app.app_state.search_results_local.contains(&original_len));
    assert!(app
        .app_state
        .search_match_positions
        .contains_key(&original_len));
    assert!(app.app_state.search_results_server.is_empty());
    assert!(app.drain_actions().is_empty());
}
