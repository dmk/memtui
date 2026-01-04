//! Effects - declarative side effects emitted by the reducer.
//!
//! Effects are handled by the effect handler which uses TaskManager
//! to spawn async tasks and return result Actions.

use crate::backend::Backend;
use crate::types::ConnectionConfig;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle to a backend connection for use in effects.
pub type BackendHandle = Arc<RwLock<Box<dyn Backend>>>;

/// Side effects emitted by the reducer.
///
/// Each effect represents an async operation that will be handled
/// by the effect handler using TaskManager.
#[derive(Clone)]
pub enum Effect {
    /// Connect to a backend server.
    Connect {
        id: String,
        config: ConnectionConfig,
    },

    /// Disconnect from a backend server.
    Disconnect { id: String },

    /// Load initial keys from the backend.
    LoadKeys {
        backend: BackendHandle,
        chunk_size: usize,
    },

    /// Load more keys around a center index (pagination).
    LoadMoreKeys {
        backend: BackendHandle,
        cursor: Option<String>,
        chunk_size: usize,
        center: usize,
    },

    /// Load value for a specific key (debounced).
    LoadValue {
        backend: BackendHandle,
        key_name: String,
    },

    /// Server-side search for keys.
    SearchServer {
        backend: BackendHandle,
        pattern: String,
    },

    /// Execute a raw backend command.
    ExecuteRaw {
        backend: BackendHandle,
        command: String,
        display_command: String,
    },

    /// Delete a key from the backend.
    DeleteKey {
        backend: BackendHandle,
        key_name: String,
    },

    /// Trigger local search (synchronous, but emitted as effect for consistency).
    TriggerSearch,
}

impl fmt::Debug for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect { id, config } => f
                .debug_struct("Connect")
                .field("id", id)
                .field("config", config)
                .finish(),
            Self::Disconnect { id } => f.debug_struct("Disconnect").field("id", id).finish(),
            Self::LoadKeys { chunk_size, .. } => f
                .debug_struct("LoadKeys")
                .field("chunk_size", chunk_size)
                .finish_non_exhaustive(),
            Self::LoadMoreKeys {
                cursor,
                chunk_size,
                center,
                ..
            } => f
                .debug_struct("LoadMoreKeys")
                .field("cursor", cursor)
                .field("chunk_size", chunk_size)
                .field("center", center)
                .finish_non_exhaustive(),
            Self::LoadValue { key_name, .. } => f
                .debug_struct("LoadValue")
                .field("key_name", key_name)
                .finish_non_exhaustive(),
            Self::SearchServer { pattern, .. } => f
                .debug_struct("SearchServer")
                .field("pattern", pattern)
                .finish_non_exhaustive(),
            Self::ExecuteRaw {
                command,
                display_command,
                ..
            } => f
                .debug_struct("ExecuteRaw")
                .field("command", command)
                .field("display_command", display_command)
                .finish_non_exhaustive(),
            Self::DeleteKey { key_name, .. } => f
                .debug_struct("DeleteKey")
                .field("key_name", key_name)
                .finish_non_exhaustive(),
            Self::TriggerSearch => write!(f, "TriggerSearch"),
        }
    }
}
