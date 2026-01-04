mod etcd;
mod memcached;
mod mock;
mod redis;
pub mod utils;

use crate::types::{BackendType, KeyMetadata, KeyScanResult, Value};
use ratatui::text::Line;
use std::time::Duration;
use thiserror::Error;

pub use etcd::EtcdBackend;
pub use memcached::MemcachedBackend;
pub use mock::MockBackend;
pub use redis::RedisBackend;

/// Formatted output from a backend command
#[derive(Debug, Clone)]
pub enum FormattedOutput {
    /// Plain text lines (for INFO, simple strings, etc.)
    Text(Vec<Line<'static>>),
    /// Table with pre-styled rows - each row is multiple styled lines
    Table { rows: Vec<Vec<Line<'static>>> },
}

/// Format backend output with syntax highlighting based on backend type.
/// This is a convenience function that dispatches to the correct backend formatter.
pub fn format_backend_output(backend_type: BackendType, text: &str) -> FormattedOutput {
    match backend_type {
        BackendType::Redis => redis::format_redis_output(text),
        BackendType::Memcached => memcached::format_memcached_output(text),
        _ => FormattedOutput::Text(text.lines().map(|l| Line::raw(l.to_string())).collect()),
    }
}

/// Main abstraction for all backend stores
#[async_trait::async_trait]
pub trait Backend: Send + Sync + std::fmt::Debug {
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

    /// Search keys with pattern - backends implement this optimally
    /// Redis: Uses SCAN MATCH efficiently (server-side)
    /// Memcached: Falls back to client-side filtering of cached keys
    async fn search_keys(&self, pattern: &str, limit: usize)
        -> Result<KeyScanResult, BackendError>;

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

    // ===== Output Formatting =====

    /// Format raw command output with syntax highlighting.
    /// Each backend can override this to colorize its protocol-specific output.
    /// Default implementation returns plain text lines.
    fn format_output(&self, text: &str) -> Vec<Line<'static>> {
        text.lines().map(|l| Line::raw(l.to_string())).collect()
    }
}

/// Backend capabilities (feature flags)
#[derive(Debug, Clone, Copy)]
pub struct BackendCapabilities {
    pub supports_ttl: bool,
    pub supports_scan: bool,
    pub supports_raw_commands: bool,
    pub supports_batch_get: bool,
    /// Whether the backend can efficiently search keys server-side (Redis: true, Memcached: false)
    pub supports_efficient_pattern_search: bool,
    /// Whether the backend supports displaying key types (Redis: true, Memcached/etcd: false)
    pub supports_type_display: bool,
    /// Whether the backend supports efficient server-side expiry display
    pub supports_expiry_display: bool,
    /// Whether the backend has limited key listing (e.g. Memcached)
    pub has_limited_key_listing: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test MockBackend basic functionality (no external services required)
    #[tokio::test]
    async fn test_connect_disconnect() {
        let mut backend = MockBackend::new(false);

        // Initially not connected
        let ping_result = backend.ping().await;
        assert!(ping_result.is_err());

        // Connect
        let conn_info = backend.connect().await.unwrap();
        assert!(conn_info.connected);
        assert_eq!(conn_info.server_version, "7.0.0-mock");

        // Ping should work now
        let duration = backend.ping().await.unwrap();
        assert!(duration.as_millis() < 100);

        // Disconnect
        backend.disconnect().await.unwrap();

        // Ping should fail after disconnect
        let ping_result = backend.ping().await;
        assert!(ping_result.is_err());
    }

    #[tokio::test]
    async fn test_info() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let info = backend.info().await.unwrap();
        assert_eq!(info.version, "7.0.0-mock");
        assert!(info.uptime.as_secs() > 0);
        assert!(info.memory_used > 0);
        assert!(info.memory_max.is_some());
        assert_eq!(info.clients_connected, 5);
        assert_eq!(info.keys_total, 4);
    }

    #[tokio::test]
    async fn test_capabilities() {
        let backend = MockBackend::new(false);
        let caps = backend.capabilities();

        assert!(caps.supports_ttl);
        assert!(caps.supports_scan);
        assert!(caps.supports_raw_commands);
        assert!(caps.supports_batch_get);
        assert!(caps.supports_efficient_pattern_search);
    }

    #[tokio::test]
    async fn test_backend_type() {
        let backend = MockBackend::new(false);
        assert_eq!(backend.backend_type(), BackendType::Redis);
    }

    #[tokio::test]
    async fn test_scan_keys_all() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.scan_keys(None, None, 100).await.unwrap();
        assert_eq!(result.keys.len(), 4);
        assert!(!result.has_more);
        assert!(result.cursor.is_none());

        // Verify key names
        let key_names: Vec<String> = result.keys.iter().map(|k| k.name.clone()).collect();
        assert!(key_names.contains(&"user:123".to_string()));
        assert!(key_names.contains(&"user:456".to_string()));
        assert!(key_names.contains(&"session:abc".to_string()));
        assert!(key_names.contains(&"cache:config".to_string()));
    }

    #[tokio::test]
    async fn test_scan_keys_with_pattern() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.scan_keys(Some("user:"), None, 100).await.unwrap();
        assert_eq!(result.keys.len(), 2);

        let key_names: Vec<String> = result.keys.iter().map(|k| k.name.clone()).collect();
        assert!(key_names.contains(&"user:123".to_string()));
        assert!(key_names.contains(&"user:456".to_string()));
    }

    #[tokio::test]
    async fn test_key_count() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let count = backend.key_count(None).await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_key_info() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let info = backend.key_info("user:123").await.unwrap();
        assert_eq!(info.name, "user:123");
        assert_eq!(info.value_type, crate::types::ValueType::Json);
        assert!(info.size_bytes > 0);
        assert!(info.ttl.is_some());
    }

    #[tokio::test]
    async fn test_key_info_not_found() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.key_info("nonexistent").await;
        assert!(matches!(result, Err(BackendError::KeyNotFound(_))));
    }

    #[tokio::test]
    async fn test_get() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        // Test JSON key
        let value = backend.get("user:123").await.unwrap();
        assert!(!value.data.is_empty());
        assert_eq!(value.value_type, crate::types::ValueType::Json);
        assert!(String::from_utf8_lossy(&value.data).contains("Alice"));

        // Test string key
        let value = backend.get("session:abc").await.unwrap();
        assert!(!value.data.is_empty());
        assert_eq!(value.value_type, crate::types::ValueType::String);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.get("nonexistent").await;
        assert!(matches!(result, Err(BackendError::KeyNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_many() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let keys = vec![
            "user:123".to_string(),
            "user:456".to_string(),
            "session:abc".to_string(),
        ];

        let results = backend.get_many(&keys).await.unwrap();
        assert_eq!(results.len(), 3);

        // Verify we got all the keys
        let result_keys: Vec<String> = results.iter().map(|(k, _)| k.clone()).collect();
        assert!(result_keys.contains(&"user:123".to_string()));
        assert!(result_keys.contains(&"user:456".to_string()));
        assert!(result_keys.contains(&"session:abc".to_string()));
    }

    #[tokio::test]
    async fn test_get_many_with_missing() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let keys = vec![
            "user:123".to_string(),
            "nonexistent".to_string(),
            "user:456".to_string(),
        ];

        let results = backend.get_many(&keys).await.unwrap();
        // Only existing keys should be returned
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_set() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        // Set without TTL
        let result = backend.set("newkey", b"newvalue", None).await;
        assert!(result.is_ok());

        // Set with TTL
        let result = backend
            .set("newkey2", b"value2", Some(Duration::from_secs(60)))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_read_only() {
        let mut backend = MockBackend::new(true); // Read-only mode
        backend.connect().await.unwrap();

        let result = backend.set("key", b"value", None).await;
        assert!(matches!(result, Err(BackendError::ReadOnly)));
    }

    #[tokio::test]
    async fn test_mock_backend_set_not_connected() {
        let backend = MockBackend::new(false);

        let result = backend.set("key", b"value", None).await;
        assert!(matches!(result, Err(BackendError::ConnectionError(_))));
    }

    #[tokio::test]
    async fn test_mock_backend_delete() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.delete("user:123").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_backend_delete_read_only() {
        let mut backend = MockBackend::new(true); // Read-only mode
        backend.connect().await.unwrap();

        let result = backend.delete("user:123").await;
        assert!(matches!(result, Err(BackendError::ReadOnly)));
    }

    #[tokio::test]
    async fn test_mock_backend_delete_not_connected() {
        let backend = MockBackend::new(false);

        let result = backend.delete("key").await;
        assert!(matches!(result, Err(BackendError::ConnectionError(_))));
    }

    #[tokio::test]
    async fn test_mock_backend_execute_raw() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        let result = backend.execute_raw("PING").await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.output.contains("PING"));
        assert!(result.error_message.is_none());
        assert!(result.duration.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_mock_backend_execute_raw_not_connected() {
        let backend = MockBackend::new(false);

        let result = backend.execute_raw("PING").await;
        assert!(matches!(result, Err(BackendError::ConnectionError(_))));
    }

    #[tokio::test]
    async fn test_mock_backend_multiple_operations() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        // Test a sequence of operations
        let info = backend.info().await.unwrap();
        assert!(!info.version.is_empty());

        let keys = backend.scan_keys(None, None, 10).await.unwrap();
        assert!(!keys.keys.is_empty());

        let value = backend.get("user:123").await.unwrap();
        assert!(!value.data.is_empty());

        let key_info = backend.key_info("user:123").await.unwrap();
        assert_eq!(key_info.name, "user:123");

        let count = backend.key_count(None).await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_mock_backend_json_value_types() {
        let mut backend = MockBackend::new(false);
        backend.connect().await.unwrap();

        // JSON keys should have Json value type
        let value = backend.get("user:123").await.unwrap();
        assert_eq!(value.value_type, crate::types::ValueType::Json);

        let value = backend.get("cache:config").await.unwrap();
        assert_eq!(value.value_type, crate::types::ValueType::Json);

        // Non-JSON keys should have String value type
        let value = backend.get("session:abc").await.unwrap();
        assert_eq!(value.value_type, crate::types::ValueType::String);
    }
}
