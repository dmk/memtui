//! Effect handler - processes effects emitted by the reducer.
//!
//! Uses TaskManager to spawn async tasks with automatic cancellation
//! and debouncing support.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tui_dispatch::tasks::TaskManager;

use crate::action::Action;
use crate::app::AppState;
use crate::backend::{Backend, EtcdBackend, MemcachedBackend, RedisBackend};
use crate::config::Config;
use crate::effect::Effect;
use crate::search::fuzzy_search_keys_with_positions;
use crate::task_keys;
use crate::types::BackendType;

/// Handle an effect by spawning the appropriate async task.
///
/// Uses TaskManager for automatic cancellation of previous tasks
/// with the same key, and debouncing for value loads.
pub fn handle_effect(
    effect: Effect,
    tasks: &mut TaskManager<Action>,
    app_state: &AppState,
    config: &Config,
) {
    match effect {
        Effect::Connect {
            id,
            config: conn_config,
        } => {
            let task_key = format!("{}:{}", task_keys::CONNECT, id);
            tasks.spawn(task_key, async move {
                let mut backend: Box<dyn Backend> = match conn_config.backend_type {
                    BackendType::Redis => Box::new(RedisBackend::new(conn_config.clone())),
                    BackendType::Memcached => Box::new(MemcachedBackend::new(conn_config.clone())),
                    BackendType::Etcd => Box::new(EtcdBackend::new(conn_config.clone())),
                };

                match backend.connect().await {
                    Ok(_) => {
                        let caps = backend.capabilities();
                        let backend_arc = Arc::new(RwLock::new(backend));
                        Action::DidConnect(conn_config.id, backend_arc, caps)
                    }
                    Err(e) => Action::DidFailConnect(conn_config.id, e.to_string()),
                }
            });
        }

        Effect::Disconnect { id } => {
            // Disconnect is handled synchronously in the reducer
            // This effect is for any cleanup tasks if needed
            tasks.spawn(format!("disconnect:{}", id), async move {
                Action::DidDisconnect(id)
            });
        }

        Effect::LoadKeys {
            backend,
            chunk_size,
        } => {
            tasks.spawn(task_keys::SCAN_KEYS, async move {
                let backend = backend.read().await;
                let total = backend.key_count(None).await.ok();

                match backend.scan_keys(None, None, chunk_size).await {
                    Ok(result) => Action::DidScanKeys {
                        keys: result.keys,
                        cursor: result.cursor,
                        has_more: result.has_more,
                        total_count: total,
                        reset: true,
                        center: None,
                    },
                    Err(e) => Action::DidFailScanKeys(e.to_string()),
                }
            });
        }

        Effect::LoadMoreKeys {
            backend,
            cursor,
            chunk_size,
            center,
        } => {
            tasks.spawn(task_keys::SCAN_KEYS, async move {
                let backend = backend.read().await;
                match backend.scan_keys(None, cursor, chunk_size).await {
                    Ok(result) => Action::DidScanKeys {
                        keys: result.keys,
                        cursor: result.cursor,
                        has_more: result.has_more,
                        total_count: None,
                        reset: false,
                        center: Some(center),
                    },
                    Err(e) => Action::DidFailScanKeys(e.to_string()),
                }
            });
        }

        Effect::LoadValue { backend, key_name } => {
            let debounce = config.data.value_load_debounce;
            tasks.debounce(task_keys::LOAD_VALUE, debounce, async move {
                let backend = backend.read().await;
                match backend.get(&key_name).await {
                    Ok(val) => Action::DidLoadValue {
                        value: val,
                        token: 0,
                    },
                    Err(e) => Action::DidFailLoadValue(e.to_string()),
                }
            });
        }

        Effect::SearchServer { backend, pattern } => {
            // Debounce server search to avoid excessive requests
            let debounce = Duration::from_millis(200);
            tasks.debounce(task_keys::SEARCH_SERVER, debounce, async move {
                let backend = backend.read().await;
                let search_pattern = format!("*{}*", pattern);
                match backend.search_keys(&search_pattern, 100).await {
                    Ok(result) => Action::DidSearchServer { result, token: 0 },
                    Err(_) => Action::DidSearchServer {
                        result: crate::types::KeyScanResult {
                            keys: vec![],
                            cursor: None,
                            has_more: false,
                        },
                        token: 0,
                    },
                }
            });
        }

        Effect::ExecuteRaw {
            backend,
            command,
            display_command,
        } => {
            tasks.spawn(task_keys::RAW_COMMAND, async move {
                let backend = backend.read().await;
                let backend_type = Some(backend.backend_type());
                match backend.execute_raw(&command).await {
                    Ok(result) => {
                        let output = if let Some(err) = result.error_message.as_deref() {
                            if result.output.is_empty() {
                                err.to_string()
                            } else {
                                format!("{}\n{}", result.output, err)
                            }
                        } else {
                            result.output
                        };
                        Action::CmdLineSetCommandResult {
                            command: display_command,
                            output,
                            status: result.status,
                            backend_type,
                        }
                    }
                    Err(e) => Action::CmdLineSetCommandResult {
                        command: display_command,
                        output: format!("Command failed: {}", e),
                        status: crate::backend::CommandStatus::Error,
                        backend_type,
                    },
                }
            });
        }

        Effect::DeleteKey { backend, key_name } => {
            tasks.spawn(task_keys::DELETE_KEY, async move {
                let backend = backend.read().await;
                match backend.delete(&key_name).await {
                    Ok(_) => Action::LoadKeys,
                    Err(e) => Action::Error(format!("Delete failed: {}", e)),
                }
            });
        }

        Effect::TriggerSearch => {
            // Local search is synchronous - run it inline and emit result
            // This is a special case where we need access to app_state
            trigger_local_search(tasks, app_state);
        }
    }
}

/// Trigger local fuzzy search and schedule server search.
///
/// Local search runs synchronously and sends results immediately.
/// Server search is debounced via TaskManager.
fn trigger_local_search(tasks: &mut TaskManager<Action>, app_state: &AppState) {
    let query = app_state.cmdline_buffer.clone();

    if query.is_empty() {
        return;
    }

    // Run local fuzzy search immediately (synchronous)
    let keys = &app_state.keys;
    let search_result = fuzzy_search_keys_with_positions(keys, &query);

    // Send local search results directly via task (completes immediately)
    let indices = search_result.indices;
    let match_positions = search_result.match_positions;
    tasks.spawn("search_local", async move {
        Action::DidSearchLocal {
            indices,
            match_positions,
            token: 0,
        }
    });

    // Schedule server search (debounced)
    if let Some(backend) = app_state.connection_manager.get_active_backend_handle() {
        let pattern = query.clone();
        let debounce = Duration::from_millis(200);
        tasks.debounce(task_keys::SEARCH_SERVER, debounce, async move {
            let backend = backend.read().await;
            let search_pattern = format!("*{}*", pattern);
            match backend.search_keys(&search_pattern, 100).await {
                Ok(result) => Action::DidSearchServer { result, token: 0 },
                Err(_) => Action::DidSearchServer {
                    result: crate::types::KeyScanResult {
                        keys: vec![],
                        cursor: None,
                        has_more: false,
                    },
                    token: 0,
                },
            }
        });
    }
}
