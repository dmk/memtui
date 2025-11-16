use super::{
    Backend, BackendCapabilities, BackendError, CommandStatus, ConnectionInfo, RawCommandResult,
    ServerInfo,
};
use crate::types::{
    Auth, BackendType, ConnectionConfig, KeyMetadata, KeyScanResult, Value, ValueType,
};
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use std::time::{Duration, SystemTime};

/// Redis backend implementation
pub struct RedisBackend {
    config: ConnectionConfig,
    connection: Option<ConnectionManager>,
}

impl RedisBackend {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    /// Build Redis connection URL from config
    fn build_connection_url(&self) -> String {
        let scheme = if self.config.tls.as_ref().map(|t| t.enabled).unwrap_or(false) {
            "rediss"
        } else {
            "redis"
        };

        let auth_part = match &self.config.auth {
            Some(Auth::UserPassword { username, password }) => {
                format!("{}:{}@", username, password)
            }
            Some(Auth::Token(token)) => {
                format!(":{}@", token)
            }
            _ => String::new(),
        };

        let db = self.config.database.as_deref().unwrap_or("0");

        format!(
            "{}://{}{}:{}/{}",
            scheme, auth_part, self.config.host, self.config.port, db
        )
    }

    /// Get a reference to the connection
    fn get_connection(&self) -> Result<&ConnectionManager, BackendError> {
        self.connection
            .as_ref()
            .ok_or_else(|| BackendError::ConnectionError("Not connected".to_string()))
    }

    /// Convert Redis error to BackendError
    fn convert_error(err: RedisError) -> BackendError {
        match err.kind() {
            redis::ErrorKind::AuthenticationFailed => {
                BackendError::AuthenticationError(err.to_string())
            }
            redis::ErrorKind::IoError | redis::ErrorKind::ClientError => {
                BackendError::ConnectionError(err.to_string())
            }
            redis::ErrorKind::TypeError => BackendError::InvalidCommand(err.to_string()),
            _ => BackendError::Internal(err.to_string()),
        }
    }

    /// Map Redis type string to ValueType
    fn map_redis_type(type_str: &str) -> ValueType {
        match type_str {
            "string" => ValueType::String,
            "list" => ValueType::List,
            "set" => ValueType::Set,
            "zset" => ValueType::SortedSet,
            "hash" => ValueType::Hash,
            _ => ValueType::Unknown,
        }
    }

    /// Detect if a string value is JSON
    fn is_json(data: &[u8]) -> bool {
        if let Ok(s) = std::str::from_utf8(data) {
            let trimmed = s.trim();
            (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        } else {
            false
        }
    }

    /// Parse Redis INFO response into a structured format
    fn parse_info(info_str: &str) -> Result<ServerInfo, BackendError> {
        let mut version = String::new();
        let mut uptime_secs = 0u64;
        let mut memory_used = 0u64;
        let mut memory_max = None;
        let mut clients_connected = 0usize;
        let mut keys_total = 0u64;

        for line in info_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                match key {
                    "redis_version" => version = value.to_string(),
                    "uptime_in_seconds" => {
                        uptime_secs = value.parse().unwrap_or(0);
                    }
                    "used_memory" => {
                        memory_used = value.parse().unwrap_or(0);
                    }
                    "maxmemory" => {
                        let max = value.parse().unwrap_or(0);
                        if max > 0 {
                            memory_max = Some(max);
                        }
                    }
                    "connected_clients" => {
                        clients_connected = value.parse().unwrap_or(0);
                    }
                    _ if key.starts_with("db") => {
                        // Parse db0:keys=123,expires=45
                        if let Some(keys_part) = value.split(',').next()
                            && let Some(count) = keys_part.strip_prefix("keys=")
                        {
                            keys_total += count.parse::<u64>().unwrap_or(0);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ServerInfo {
            version,
            uptime: Duration::from_secs(uptime_secs),
            memory_used,
            memory_max,
            clients_connected,
            keys_total,
        })
    }
}

#[async_trait::async_trait]
impl Backend for RedisBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Redis
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_ttl: true,
            supports_scan: true,
            supports_raw_commands: true,
            supports_batch_get: true,
        }
    }

    async fn connect(&mut self) -> Result<ConnectionInfo, BackendError> {
        let url = self.build_connection_url();

        let client =
            redis::Client::open(url).map_err(|e| BackendError::ConnectionError(e.to_string()))?;

        let connection = tokio::time::timeout(self.config.timeout, ConnectionManager::new(client))
            .await
            .map_err(|_| BackendError::Timeout)?
            .map_err(Self::convert_error)?;

        // Test the connection with INFO command
        let mut conn = connection.clone();
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        let mut version = "unknown".to_string();
        for line in info.lines() {
            if let Some(v) = line.strip_prefix("redis_version:") {
                version = v.trim().to_string();
                break;
            }
        }

        self.connection = Some(connection);

        Ok(ConnectionInfo {
            connected: true,
            server_version: version,
            address: format!("{}:{}", self.config.host, self.config.port),
        })
    }

    async fn disconnect(&mut self) -> Result<(), BackendError> {
        self.connection = None;
        Ok(())
    }

    async fn ping(&self) -> Result<Duration, BackendError> {
        let mut conn = self.get_connection()?.clone();
        let start = std::time::Instant::now();

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        Ok(start.elapsed())
    }

    async fn info(&self) -> Result<ServerInfo, BackendError> {
        let mut conn = self.get_connection()?.clone();

        let info_str: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        Self::parse_info(&info_str)
    }

    async fn scan_keys(
        &self,
        pattern: Option<&str>,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<KeyScanResult, BackendError> {
        let mut conn = self.get_connection()?.clone();

        let cursor_val: u64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
        let pattern = pattern.unwrap_or("*");

        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor_val)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(limit)
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        // Get metadata for each key
        let mut key_metadata = Vec::new();
        for key in keys {
            if let Ok(metadata) = self.key_info(&key).await {
                key_metadata.push(metadata);
            }
        }

        Ok(KeyScanResult {
            keys: key_metadata,
            cursor: if next_cursor > 0 {
                Some(next_cursor.to_string())
            } else {
                None
            },
            has_more: next_cursor > 0,
        })
    }

    async fn key_count(&self, pattern: Option<&str>) -> Result<u64, BackendError> {
        let mut conn = self.get_connection()?.clone();

        match pattern {
            None | Some("*") => {
                // Fast path: use DBSIZE
                let count: u64 = redis::cmd("DBSIZE")
                    .query_async(&mut conn)
                    .await
                    .map_err(Self::convert_error)?;
                Ok(count)
            }
            Some(pattern) => {
                // Slow path: scan all keys
                let mut cursor = 0u64;
                let mut count = 0u64;

                loop {
                    let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(pattern)
                        .arg("COUNT")
                        .arg(1000)
                        .query_async(&mut conn)
                        .await
                        .map_err(Self::convert_error)?;

                    count += keys.len() as u64;

                    if next_cursor == 0 {
                        break;
                    }
                    cursor = next_cursor;
                }

                Ok(count)
            }
        }
    }

    async fn key_info(&self, key: &str) -> Result<KeyMetadata, BackendError> {
        let mut conn = self.get_connection()?.clone();

        // Check if key exists
        let exists: bool = conn.exists(key).await.map_err(Self::convert_error)?;
        if !exists {
            return Err(BackendError::KeyNotFound(key.to_string()));
        }

        // Get type
        let type_str: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        let value_type = Self::map_redis_type(&type_str);

        // Get TTL
        let ttl_secs: i64 = redis::cmd("TTL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        let ttl = if ttl_secs > 0 {
            Some(Duration::from_secs(ttl_secs as u64))
        } else {
            None
        };

        // Get memory usage (if available, requires Redis 4.0+)
        let size_bytes: u64 = redis::cmd("MEMORY")
            .arg("USAGE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        // Get encoding
        let encoding: String = redis::cmd("OBJECT")
            .arg("ENCODING")
            .arg(key)
            .query_async(&mut conn)
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(KeyMetadata {
            name: key.to_string(),
            value_type,
            size_bytes,
            ttl,
            last_accessed: Some(SystemTime::now()),
            encoding: Some(encoding),
        })
    }

    async fn get(&self, key: &str) -> Result<Value, BackendError> {
        let mut conn = self.get_connection()?.clone();

        // Get the key type first
        let type_str: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        // Check if key exists
        if type_str == "none" {
            return Err(BackendError::KeyNotFound(key.to_string()));
        }

        let value_type = Self::map_redis_type(&type_str);

        // For simple string values
        if matches!(value_type, ValueType::String) {
            let data: Vec<u8> = conn.get(key).await.map_err(|e| {
                if e.kind() == redis::ErrorKind::TypeError {
                    BackendError::KeyNotFound(key.to_string())
                } else {
                    Self::convert_error(e)
                }
            })?;

            // Try to detect if it's JSON
            let final_type = if Self::is_json(&data) {
                ValueType::Json
            } else if data.iter().all(|&b| b.is_ascii()) {
                ValueType::String
            } else {
                ValueType::Binary
            };

            Ok(Value {
                data,
                value_type: final_type,
                encoding: Some("utf8".to_string()),
            })
        } else {
            // For complex types, serialize as JSON
            let serialized = match value_type {
                ValueType::List => {
                    let items: Vec<String> = redis::cmd("LRANGE")
                        .arg(key)
                        .arg(0)
                        .arg(-1)
                        .query_async(&mut conn)
                        .await
                        .map_err(Self::convert_error)?;
                    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
                }
                ValueType::Set => {
                    let items: Vec<String> = redis::cmd("SMEMBERS")
                        .arg(key)
                        .query_async(&mut conn)
                        .await
                        .map_err(Self::convert_error)?;
                    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
                }
                ValueType::Hash => {
                    let items: std::collections::HashMap<String, String> = redis::cmd("HGETALL")
                        .arg(key)
                        .query_async(&mut conn)
                        .await
                        .map_err(Self::convert_error)?;
                    serde_json::to_string(&items).unwrap_or_else(|_| "{}".to_string())
                }
                ValueType::SortedSet => {
                    let items: Vec<(String, f64)> = redis::cmd("ZRANGE")
                        .arg(key)
                        .arg(0)
                        .arg(-1)
                        .arg("WITHSCORES")
                        .query_async(&mut conn)
                        .await
                        .map_err(Self::convert_error)?;
                    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
                }
                _ => String::from("{}"),
            };

            Ok(Value {
                data: serialized.into_bytes(),
                value_type: ValueType::Json,
                encoding: Some("utf8".to_string()),
            })
        }
    }

    async fn get_many(&self, keys: &[String]) -> Result<Vec<(String, Value)>, BackendError> {
        let mut conn = self.get_connection()?.clone();

        // Use MGET for efficiency (only works for string keys)
        let values: Vec<Option<Vec<u8>>> = redis::cmd("MGET")
            .arg(keys)
            .query_async(&mut conn)
            .await
            .map_err(Self::convert_error)?;

        let mut results = Vec::new();
        for (key, maybe_value) in keys.iter().zip(values.iter()) {
            if let Some(data) = maybe_value {
                let final_type = if Self::is_json(data) {
                    ValueType::Json
                } else if data.iter().all(|&b| b.is_ascii()) {
                    ValueType::String
                } else {
                    ValueType::Binary
                };

                results.push((
                    key.clone(),
                    Value {
                        data: data.clone(),
                        value_type: final_type,
                        encoding: Some("utf8".to_string()),
                    },
                ));
            }
        }

        Ok(results)
    }

    async fn set(
        &self,
        key: &str,
        value: &[u8],
        ttl: Option<Duration>,
    ) -> Result<(), BackendError> {
        if self.config.read_only {
            return Err(BackendError::ReadOnly);
        }

        let mut conn = self.get_connection()?.clone();

        if let Some(ttl) = ttl {
            let _: () = conn
                .set_ex(key, value, ttl.as_secs())
                .await
                .map_err(Self::convert_error)?;
        } else {
            let _: () = conn.set(key, value).await.map_err(Self::convert_error)?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, BackendError> {
        if self.config.read_only {
            return Err(BackendError::ReadOnly);
        }

        let mut conn = self.get_connection()?.clone();

        let deleted: i32 = conn.del(key).await.map_err(Self::convert_error)?;

        Ok(deleted > 0)
    }

    async fn execute_raw(&self, command: &str) -> Result<RawCommandResult, BackendError> {
        let mut conn = self.get_connection()?.clone();

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(BackendError::InvalidCommand("Empty command".to_string()));
        }

        let start = std::time::Instant::now();

        let mut cmd = redis::cmd(parts[0]);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }

        match cmd.query_async::<redis::Value>(&mut conn).await {
            Ok(value) => {
                let output = format!("{:?}", value);
                Ok(RawCommandResult {
                    output,
                    status: CommandStatus::Success,
                    duration: start.elapsed(),
                    error_message: None,
                })
            }
            Err(err) => Ok(RawCommandResult {
                output: String::new(),
                status: CommandStatus::Error,
                duration: start.elapsed(),
                error_message: Some(err.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Auth, TlsConfig};

    fn create_test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test".to_string(),
            name: "Test Redis".to_string(),
            backend_type: BackendType::Redis,
            host: "localhost".to_string(),
            port: 6379,
            auth: None,
            database: Some("0".to_string()),
            tls: None,
            timeout: Duration::from_secs(5),
            read_only: false,
        }
    }

    #[test]
    fn test_build_connection_url_basic() {
        let backend = RedisBackend::new(create_test_config());
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://localhost:6379/0");
    }

    #[test]
    fn test_build_connection_url_with_password() {
        let mut config = create_test_config();
        config.auth = Some(Auth::Token("secret123".to_string()));
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://:secret123@localhost:6379/0");
    }

    #[test]
    fn test_build_connection_url_with_username_password() {
        let mut config = create_test_config();
        config.auth = Some(Auth::UserPassword {
            username: "admin".to_string(),
            password: "pass123".to_string(),
        });
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://admin:pass123@localhost:6379/0");
    }

    #[test]
    fn test_build_connection_url_with_tls() {
        let mut config = create_test_config();
        config.tls = Some(TlsConfig {
            enabled: true,
            ca_cert_path: None,
            verify_hostname: true,
        });
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "rediss://localhost:6379/0");
    }

    #[test]
    fn test_build_connection_url_different_database() {
        let mut config = create_test_config();
        config.database = Some("5".to_string());
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://localhost:6379/5");
    }

    #[test]
    fn test_build_connection_url_no_database() {
        let mut config = create_test_config();
        config.database = None;
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://localhost:6379/0");
    }

    #[test]
    fn test_build_connection_url_custom_port() {
        let mut config = create_test_config();
        config.port = 6380;
        let backend = RedisBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "redis://localhost:6380/0");
    }

    #[test]
    fn test_map_redis_type() {
        assert_eq!(RedisBackend::map_redis_type("string"), ValueType::String);
        assert_eq!(RedisBackend::map_redis_type("list"), ValueType::List);
        assert_eq!(RedisBackend::map_redis_type("set"), ValueType::Set);
        assert_eq!(RedisBackend::map_redis_type("zset"), ValueType::SortedSet);
        assert_eq!(RedisBackend::map_redis_type("hash"), ValueType::Hash);
        assert_eq!(RedisBackend::map_redis_type("unknown"), ValueType::Unknown);
        assert_eq!(RedisBackend::map_redis_type("stream"), ValueType::Unknown);
    }

    #[test]
    fn test_is_json_object() {
        assert!(RedisBackend::is_json(b"{\"key\": \"value\"}"));
        assert!(RedisBackend::is_json(b"  {\"key\": \"value\"}  "));
        assert!(RedisBackend::is_json(b"{}"));
    }

    #[test]
    fn test_is_json_array() {
        assert!(RedisBackend::is_json(b"[1, 2, 3]"));
        assert!(RedisBackend::is_json(b"  [1, 2, 3]  "));
        assert!(RedisBackend::is_json(b"[]"));
    }

    #[test]
    fn test_is_not_json() {
        assert!(!RedisBackend::is_json(b"plain string"));
        assert!(!RedisBackend::is_json(b"123"));
        assert!(!RedisBackend::is_json(b"{incomplete"));
        assert!(!RedisBackend::is_json(b"[incomplete"));
        assert!(!RedisBackend::is_json(b""));
    }

    #[test]
    fn test_is_json_invalid_utf8() {
        // Invalid UTF-8 should return false
        assert!(!RedisBackend::is_json(&[0xFF, 0xFE]));
    }

    #[test]
    fn test_parse_info_basic() {
        let info_str = r#"# Server
redis_version:7.0.0
uptime_in_seconds:86400
# Memory
used_memory:10485760
maxmemory:104857600
# Clients
connected_clients:5
# Keyspace
db0:keys=100,expires=20
db1:keys=50,expires=10
"#;

        let result = RedisBackend::parse_info(info_str).unwrap();
        assert_eq!(result.version, "7.0.0");
        assert_eq!(result.uptime, Duration::from_secs(86400));
        assert_eq!(result.memory_used, 10485760);
        assert_eq!(result.memory_max, Some(104857600));
        assert_eq!(result.clients_connected, 5);
        assert_eq!(result.keys_total, 150); // 100 + 50
    }

    #[test]
    fn test_parse_info_no_maxmemory() {
        let info_str = r#"redis_version:6.2.0
uptime_in_seconds:3600
used_memory:5242880
maxmemory:0
connected_clients:2
"#;

        let result = RedisBackend::parse_info(info_str).unwrap();
        assert_eq!(result.version, "6.2.0");
        assert_eq!(result.memory_max, None); // maxmemory:0 means no limit
    }

    #[test]
    fn test_parse_info_empty_string() {
        let result = RedisBackend::parse_info("").unwrap();
        assert_eq!(result.version, "");
        assert_eq!(result.uptime, Duration::from_secs(0));
        assert_eq!(result.memory_used, 0);
        assert_eq!(result.memory_max, None);
        assert_eq!(result.clients_connected, 0);
        assert_eq!(result.keys_total, 0);
    }

    #[test]
    fn test_parse_info_with_comments() {
        let info_str = r#"# This is a comment
redis_version:7.2.0
# Another comment
uptime_in_seconds:12345
"#;

        let result = RedisBackend::parse_info(info_str).unwrap();
        assert_eq!(result.version, "7.2.0");
        assert_eq!(result.uptime, Duration::from_secs(12345));
    }

    #[test]
    fn test_backend_type() {
        let backend = RedisBackend::new(create_test_config());
        assert_eq!(backend.backend_type(), BackendType::Redis);
    }

    #[test]
    fn test_capabilities() {
        let backend = RedisBackend::new(create_test_config());
        let caps = backend.capabilities();
        assert!(caps.supports_ttl);
        assert!(caps.supports_scan);
        assert!(caps.supports_raw_commands);
        assert!(caps.supports_batch_get);
    }

    #[test]
    fn test_convert_error_authentication() {
        let redis_err = RedisError::from((redis::ErrorKind::AuthenticationFailed, "auth failed"));
        let backend_err = RedisBackend::convert_error(redis_err);
        assert!(matches!(backend_err, BackendError::AuthenticationError(_)));
    }

    #[test]
    fn test_convert_error_connection() {
        let redis_err = RedisError::from((redis::ErrorKind::IoError, "connection failed"));
        let backend_err = RedisBackend::convert_error(redis_err);
        assert!(matches!(backend_err, BackendError::ConnectionError(_)));
    }

    #[test]
    fn test_convert_error_invalid_command() {
        let redis_err = RedisError::from((redis::ErrorKind::TypeError, "wrong type"));
        let backend_err = RedisBackend::convert_error(redis_err);
        assert!(matches!(backend_err, BackendError::InvalidCommand(_)));
    }

    #[test]
    fn test_get_connection_not_connected() {
        let backend = RedisBackend::new(create_test_config());
        let result = backend.get_connection();
        assert!(matches!(result, Err(BackendError::ConnectionError(_))));
    }
}
