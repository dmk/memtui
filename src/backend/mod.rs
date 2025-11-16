mod memcached;
mod mock;
mod redis;

use crate::types::{BackendType, KeyMetadata, KeyScanResult, Value};
use std::time::Duration;
use thiserror::Error;

pub use memcached::MemcachedBackend;
pub use mock::MockBackend;
pub use redis::RedisBackend;

/// Main abstraction for all backend stores
#[async_trait::async_trait]
pub trait Backend: Send + Sync {
    /// Get backend type (redis, memcached, etcd, etc.)
    fn backend_type(&self) -> BackendType;

    /// Get backend capabilities (what features are supported)
    fn capabilities(&self) -> BackendCapabilities;

    /// Connect to the backend
    async fn connect(&mut self) -> Result<ConnectionInfo, BackendError>;

    /// Disconnect gracefully
    async fn disconnect(&mut self) -> Result<(), BackendError>;

    /// Check if connection is alive
    async fn ping(&self) -> Result<Duration, BackendError>;

    /// Get server/cluster information
    async fn info(&self) -> Result<ServerInfo, BackendError>;

    // ===== Key Operations (Read-only by default) =====

    /// List keys matching a pattern (returns paginated results)
    async fn scan_keys(
        &self,
        pattern: Option<&str>,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<KeyScanResult, BackendError>;

    /// Get total key count (approximate if expensive)
    async fn key_count(&self, pattern: Option<&str>) -> Result<u64, BackendError>;

    /// Get metadata for a specific key
    async fn key_info(&self, key: &str) -> Result<KeyMetadata, BackendError>;

    /// Get value for a key
    async fn get(&self, key: &str) -> Result<Value, BackendError>;

    /// Get multiple values at once (optimized batch fetch)
    async fn get_many(&self, keys: &[String]) -> Result<Vec<(String, Value)>, BackendError>;

    // ===== Write Operations (requires write_enabled flag) =====

    /// Set a value (only if write_enabled)
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>)
    -> Result<(), BackendError>;

    /// Delete a key (only if write_enabled)
    async fn delete(&self, key: &str) -> Result<bool, BackendError>;

    // ===== Advanced Operations =====

    /// Execute a raw command (if supported)
    async fn execute_raw(&self, command: &str) -> Result<RawCommandResult, BackendError>;
}

/// Backend capabilities (feature flags)
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_ttl: bool,
    pub supports_scan: bool,
    pub supports_raw_commands: bool,
    pub supports_batch_get: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connected: bool,
    pub server_version: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub version: String,
    pub uptime: Duration,
    pub memory_used: u64,
    pub memory_max: Option<u64>,
    pub clients_connected: usize,
    pub keys_total: u64,
}

#[derive(Debug, Clone)]
pub struct RawCommandResult {
    pub output: String,
    pub status: CommandStatus,
    pub duration: Duration,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Success,
    Error,
    Timeout,
    PermissionDenied,
    InvalidCommand,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Operation not supported")]
    NotSupported,

    #[error("Write operation not allowed (read-only mode)")]
    ReadOnly,

    #[error("Timeout")]
    Timeout,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
