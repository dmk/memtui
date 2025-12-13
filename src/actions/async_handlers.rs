//! Async action handlers (connection, data loading, search)
//!
//! These handlers spawn async tasks and communicate results via the action channel.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::action::Action;
use crate::app::{AppState, ConnectionStatus};
use crate::backend::{Backend, EtcdBackend, MemcachedBackend, RedisBackend};
use crate::config::Config;
use crate::search::fuzzy_search_keys_with_positions;
use crate::types::{BackendType, KeyMetadata, Value};
use crate::ui::{Panel, UiState};
use crate::userdata;

/// Handle Connect action - spawn async connection task
pub fn handle_connect(
    app_state: &mut AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    id: String,
) {
    if let Some(config) = app_state.connection_manager.get_config(&id).cloned() {
        app_state
            .connection_manager
            .set_status(&id, ConnectionStatus::Connecting);
        let tx = action_tx.clone();

        tokio::spawn(async move {
            let mut backend: Box<dyn Backend> = match config.backend_type {
                BackendType::Redis => Box::new(RedisBackend::new(config.clone())),
                BackendType::Memcached => Box::new(MemcachedBackend::new(config.clone())),
                BackendType::Etcd => Box::new(EtcdBackend::new(config.clone())),
            };

            match backend.connect().await {
                Ok(_) => {
                    let backend_arc = Arc::new(RwLock::new(backend));
                    let _ = tx.send(Action::DidConnect(config.id, backend_arc));
                }
                Err(e) => {
                    let _ = tx.send(Action::DidFailConnect(config.id, e.to_string()));
                }
            }
        });
    }
}

/// Handle LoadKeys action - spawn async key loading task
pub fn handle_load_keys(app_state: &mut AppState, action_tx: &mpsc::UnboundedSender<Action>) {
    app_state.is_loading_keys = true;
    app_state.reset_pagination();

    if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
        let tx = action_tx.clone();
        let chunk_size = app_state.keys_per_chunk;

        tokio::spawn(async move {
            let backend = backend.read().await;
            let total = backend.key_count(None).await.ok();

            match backend.scan_keys(None, None, chunk_size).await {
                Ok(result) => {
                    let _ = tx.send(Action::DidScanKeys {
                        keys: result.keys,
                        cursor: result.cursor,
                        has_more: result.has_more,
                        total_count: total,
                        reset: true,
                        center: None,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Action::DidFailScanKeys(e.to_string()));
                }
            }
        });
    }
}

/// Handle LoadMoreKeys action - spawn async task to load more keys around a center index
pub fn handle_load_more_keys(
    app_state: &mut AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    center: usize,
) -> bool {
    if app_state.is_loading_keys || !app_state.has_more_keys {
        return false;
    }

    app_state.is_loading_keys = true;

    if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
        let tx = action_tx.clone();
        let chunk_size = app_state.keys_per_chunk;
        let cursor = app_state.keys_cursor.clone();

        tokio::spawn(async move {
            let backend = backend.read().await;
            match backend.scan_keys(None, cursor, chunk_size).await {
                Ok(result) => {
                    let _ = tx.send(Action::DidScanKeys {
                        keys: result.keys,
                        cursor: result.cursor,
                        has_more: result.has_more,
                        total_count: None,
                        reset: false,
                        center: Some(center),
                    });
                }
                Err(e) => {
                    let _ = tx.send(Action::DidFailScanKeys(e.to_string()));
                }
            }
        });
    }
    true
}

/// Handle LoadValue action - spawn async task to load a key's value
pub fn handle_load_value(
    app_state: &AppState,
    action_tx: &mpsc::UnboundedSender<Action>,
    index: usize,
    token: u64,
) {
    if let Some(Some(key)) = app_state.keys.get(index) {
        let key_name = key.name.clone();
        if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
            let tx = action_tx.clone();
            tokio::spawn(async move {
                let backend = backend.read().await;
                match backend.get(&key_name).await {
                    Ok(val) => {
                        let _ = tx.send(Action::DidLoadValue { value: val, token });
                    }
                    Err(e) => {
                        let _ = tx.send(Action::DidFailLoadValue(e.to_string()));
                    }
                }
            });
        }
    }
}

/// Trigger search - runs local fuzzy search and schedules server search
pub fn trigger_search(app_state: &mut AppState, action_tx: &mpsc::UnboundedSender<Action>) {
    app_state.search_token = app_state.search_token.wrapping_add(1);
    let token = app_state.search_token;
    let query = app_state.search_query.clone();

    if query.is_empty() {
        app_state.search_results_local.clear();
        app_state.search_results_server.clear();
        app_state.search_match_positions.clear();
        app_state.is_server_searching = false;
        return;
    }

    // Clear stale server results immediately (server search is async + debounced).
    app_state.search_results_server.clear();
    app_state.is_server_searching = false;

    // Immediate local fuzzy search (sync work).
    let search_result = fuzzy_search_keys_with_positions(&app_state.keys, &query);
    app_state.search_results_local = search_result.indices;
    app_state.search_match_positions = search_result.match_positions;

    // Keep current selection if it remains in results; otherwise pick first match.
    let should_select_first = match app_state.selected_key_index {
        Some(idx) => !app_state.search_results_local.contains(&idx),
        None => true,
    };
    if should_select_first {
        if let Some(&key_idx) = app_state.search_results_local.first() {
            let _ = action_tx.send(Action::SelectKey(key_idx));
        }
    }

    // Background server search
    if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
        app_state.is_server_searching = true;
        let tx = action_tx.clone();
        let search_query = query.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let backend = backend.read().await;
            let pattern = format!("*{}*", search_query);
            match backend.search_keys(&pattern, 100).await {
                Ok(result) => {
                    let _ = tx.send(Action::DidSearchServer { result, token });
                }
                Err(_) => {
                    let _ = tx.send(Action::DidSearchServer {
                        result: crate::types::KeyScanResult {
                            keys: vec![],
                            cursor: None,
                            has_more: false,
                        },
                        token,
                    });
                }
            }
        });
    }
}

// =============================================================================
// Result handlers - process async results and update state
// =============================================================================

/// Handle DidConnect result - register connection and trigger key loading
pub fn handle_did_connect(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    config: &Config,
    action_tx: &mpsc::UnboundedSender<Action>,
    id: String,
    backend: Arc<RwLock<Box<dyn Backend>>>,
) {
    app_state
        .connection_manager
        .register_connection(&id, backend);
    app_state.error_message = None;
    if let Ok(ids) = userdata::record_recent_connection_id_with_config(&id, config) {
        ui_state.recent_connection_ids = ids;
    }
    ui_state.active_panel = Panel::Keys;
    let _ = action_tx.send(Action::LoadKeys);
}

/// Handle DidFailConnect result
pub fn handle_did_fail_connect(app_state: &mut AppState, id: &str, error: String) {
    app_state
        .connection_manager
        .set_status(id, ConnectionStatus::Error(error.clone()));
    app_state.error_message = Some(format!("Failed to connect: {}", error));
}

/// Handle DidScanKeys result - update key list with scan results
#[allow(clippy::too_many_arguments)]
pub fn handle_did_scan_keys(
    app_state: &mut AppState,
    ui_state: &mut UiState,
    action_tx: &mpsc::UnboundedSender<Action>,
    keys: Vec<KeyMetadata>,
    cursor: Option<String>,
    has_more: bool,
    total_count: Option<u64>,
    reset: bool,
    center: Option<usize>,
) {
    if reset {
        if let Some(count) = total_count {
            app_state.total_key_count = Some(count);
            app_state.keys = vec![None; count as usize];
        } else {
            app_state.total_key_count = None;
            app_state.keys = vec![None; keys.len()];
        }
    }

    app_state.keys_cursor = cursor;
    app_state.has_more_keys = has_more;
    app_state.is_loading_keys = false;

    if reset {
        // Simple fill from start
        for (i, k) in keys.into_iter().enumerate() {
            if i < app_state.keys.len() {
                app_state.keys[i] = Some(k);
            } else if app_state.total_key_count.is_none() {
                app_state.keys.push(Some(k));
            }
        }

        // Auto-select first key if none selected
        if app_state.selected_key_index.is_none()
            && !app_state.keys.is_empty()
            && app_state.keys[0].is_some()
        {
            ui_state.key_list.select(Some(0));
            let _ = action_tx.send(Action::SelectKey(0));
        }
    } else if let Some(c) = center {
        // Smart fill around center
        let preferred = app_state.get_preferred_indices_for_filling(c, keys.len());
        let mut keys_iter = keys.into_iter();

        for idx in preferred {
            if let Some(key) = keys_iter.next() {
                if idx < app_state.keys.len() {
                    app_state.keys[idx] = Some(key);
                }
            } else {
                break;
            }
        }

        // Fill any other empty slots with remaining keys
        for key in keys_iter {
            if let Some(empty_idx) = app_state.keys.iter().position(|k| k.is_none()) {
                app_state.keys[empty_idx] = Some(key);
            }
        }
    }

    // If a search filter is active, recompute local fuzzy results so newly-loaded keys participate.
    if !app_state.search_query.is_empty() {
        let query = app_state.search_query.clone();
        let search_result = fuzzy_search_keys_with_positions(&app_state.keys, &query);
        app_state.search_results_local = search_result.indices;
        app_state.search_match_positions = search_result.match_positions;

        // Ensure selection stays on a visible (matching) key when possible.
        let should_select_first = match app_state.selected_key_index {
            Some(idx) => !app_state.search_results_local.contains(&idx),
            None => true,
        };
        if should_select_first {
            if let Some(&key_idx) = app_state.search_results_local.first() {
                ui_state.key_list.select(Some(key_idx));
                let _ = action_tx.send(Action::SelectKey(key_idx));
            }
        }
    }
}

/// Handle DidLoadValue result
pub fn handle_did_load_value(app_state: &mut AppState, value: Value, token: u64) -> bool {
    if app_state.value_request_token == token {
        app_state.selected_value = Some(value);
        true
    } else {
        false
    }
}

/// Handle DidSearchLocal result
pub fn handle_did_search_local(
    app_state: &mut AppState,
    _action_tx: &mpsc::UnboundedSender<Action>,
    indices: Vec<usize>,
    match_positions: std::collections::HashMap<usize, Vec<u32>>,
    token: u64,
) -> bool {
    if app_state.search_token == token && !app_state.search_query.is_empty() {
        app_state.search_results_local = indices;
        app_state.search_match_positions = match_positions;
        true
    } else {
        false
    }
}

/// Handle DidSearchServer result
pub fn handle_did_search_server(
    app_state: &mut AppState,
    keys: Vec<KeyMetadata>,
    token: u64,
) -> bool {
    if app_state.search_token == token {
        app_state.search_results_server = keys;
        app_state.is_server_searching = false;
        true
    } else {
        false
    }
}

/// Handle Disconnect action
pub async fn handle_disconnect(app_state: &mut AppState, ui_state: &mut UiState, id: &str) {
    let was_focus = app_state
        .connection_manager
        .get_active_id()
        .map(|active| active == id)
        .unwrap_or(false);
    let _ = app_state.connection_manager.disconnect(id).await;
    if was_focus {
        app_state.reset_pagination();
        ui_state.key_list.select(None);
        ui_state.active_panel = Panel::Keys;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueType;
    use std::time::Duration;

    fn key(name: &str) -> KeyMetadata {
        KeyMetadata {
            name: name.to_string(),
            value_type: ValueType::String,
            size_bytes: 0,
            ttl: Some(Duration::from_secs(60)),
            last_accessed: None,
            encoding: None,
        }
    }

    #[test]
    fn trigger_search_clears_stale_server_results_and_updates_local() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app_state = AppState::new();

        app_state.keys = vec![Some(key("alpha")), Some(key("beta"))];
        app_state.search_query = "alp".to_string();
        app_state.search_results_server = vec![key("stale")];

        trigger_search(&mut app_state, &tx);

        assert!(app_state.search_results_server.is_empty());
        assert_eq!(app_state.search_results_local, vec![0]);
        assert!(!app_state.is_server_searching);

        let action = rx.try_recv().expect("expected a SelectKey action");
        match action {
            Action::SelectKey(idx) => assert_eq!(idx, 0),
            _ => panic!("unexpected action: expected SelectKey"),
        }
    }

    #[test]
    fn did_scan_keys_refreshes_local_search_results() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app_state = AppState::new();
        let mut ui_state = UiState::new();

        app_state.keys = vec![Some(key("a")), None];
        app_state.total_key_count = Some(2);
        app_state.search_query = "b".to_string();
        app_state.selected_key_index = Some(0);

        handle_did_scan_keys(
            &mut app_state,
            &mut ui_state,
            &tx,
            vec![key("b")],
            None,
            false,
            None,
            false,
            Some(0),
        );

        assert_eq!(
            app_state.keys[1].as_ref().expect("key should be loaded").name,
            "b"
        );
        assert_eq!(app_state.search_results_local, vec![1]);

        let action = rx.try_recv().expect("expected a SelectKey action");
        match action {
            Action::SelectKey(idx) => assert_eq!(idx, 1),
            _ => panic!("unexpected action: expected SelectKey"),
        }
    }

    #[test]
    fn trigger_search_empty_query_clears_state_and_cancels() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app_state = AppState::new();

        app_state.search_results_local = vec![0];
        app_state.search_results_server = vec![key("stale")];
        app_state.search_match_positions.insert(0, vec![0]);
        app_state.is_server_searching = true;
        app_state.search_query.clear();

        let before_token = app_state.search_token;
        trigger_search(&mut app_state, &tx);

        assert!(app_state.search_results_local.is_empty());
        assert!(app_state.search_results_server.is_empty());
        assert!(app_state.search_match_positions.is_empty());
        assert!(!app_state.is_server_searching);
        assert_ne!(app_state.search_token, before_token);
        assert!(rx.try_recv().is_err());
    }
}
