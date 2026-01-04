//! Task keys for TaskManager.
//!
//! Each key identifies a category of task. When a new task is spawned
//! with a key that's already running, the previous task is cancelled.

/// Connect to a backend (per connection ID).
pub const CONNECT: &str = "connect";

/// Scan/load keys from backend.
pub const SCAN_KEYS: &str = "scan_keys";

/// Load value for selected key (debounced).
pub const LOAD_VALUE: &str = "load_value";

/// Server-side search.
pub const SEARCH_SERVER: &str = "search_server";

/// Execute raw command.
pub const RAW_COMMAND: &str = "raw_command";

/// Delete a key.
pub const DELETE_KEY: &str = "delete_key";
