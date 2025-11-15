use super::{
    Backend, BackendCapabilities, BackendError, CommandStatus, ConnectionInfo, RawCommandResult,
    ServerInfo,
};
use crate::types::{BackendType, KeyMetadata, KeyScanResult, Value, ValueType};
use std::time::{Duration, SystemTime};

/// Mock backend for testing and development
pub struct MockBackend {
    connected: bool,
    read_only: bool,
}

impl MockBackend {
    pub fn new(read_only: bool) -> Self {
        Self {
            connected: false,
            read_only,
        }
    }

    fn mock_keys() -> Vec<KeyMetadata> {
        vec![
            KeyMetadata {
                name: "user:123".to_string(),
                value_type: ValueType::Json,
                size_bytes: 85,
                ttl: Some(Duration::from_secs(3600)),
                last_accessed: Some(SystemTime::now()),
                encoding: Some("utf8".to_string()),
            },
            KeyMetadata {
                name: "user:456".to_string(),
                value_type: ValueType::Json,
                size_bytes: 92,
                ttl: Some(Duration::from_secs(3600)),
                last_accessed: Some(SystemTime::now()),
                encoding: Some("utf8".to_string()),
            },
            KeyMetadata {
                name: "session:abc".to_string(),
                value_type: ValueType::String,
                size_bytes: 128,
                ttl: Some(Duration::from_secs(1800)),
                last_accessed: Some(SystemTime::now()),
                encoding: Some("utf8".to_string()),
            },
            KeyMetadata {
                name: "cache:config".to_string(),
                value_type: ValueType::Json,
                size_bytes: 45,
                ttl: None,
                last_accessed: Some(SystemTime::now()),
                encoding: Some("utf8".to_string()),
            },
        ]
    }
}

#[async_trait::async_trait]
impl Backend for MockBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Redis // Pretend to be Redis
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
        self.connected = true;
        Ok(ConnectionInfo {
            connected: true,
            server_version: "7.0.0-mock".to_string(),
            address: "mock://localhost:6379".to_string(),
        })
    }

    async fn disconnect(&mut self) -> Result<(), BackendError> {
        self.connected = false;
        Ok(())
    }

    async fn ping(&self) -> Result<Duration, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }
        Ok(Duration::from_millis(1))
    }

    async fn info(&self) -> Result<ServerInfo, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        Ok(ServerInfo {
            version: "7.0.0-mock".to_string(),
            uptime: Duration::from_secs(86400),
            memory_used: 1024 * 1024 * 10,       // 10 MB
            memory_max: Some(1024 * 1024 * 100), // 100 MB
            clients_connected: 5,
            keys_total: 4,
        })
    }

    async fn scan_keys(
        &self,
        pattern: Option<&str>,
        _cursor: Option<String>,
        _limit: usize,
    ) -> Result<KeyScanResult, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        let mut keys = Self::mock_keys();

        // Simple pattern matching
        if let Some(pattern) = pattern
            && pattern != "*"
        {
            keys.retain(|k| k.name.contains(pattern));
        }

        Ok(KeyScanResult {
            keys,
            cursor: None,
            has_more: false,
        })
    }

    async fn key_count(&self, _pattern: Option<&str>) -> Result<u64, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }
        Ok(4)
    }

    async fn key_info(&self, key: &str) -> Result<KeyMetadata, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        Self::mock_keys()
            .into_iter()
            .find(|k| k.name == key)
            .ok_or_else(|| BackendError::KeyNotFound(key.to_string()))
    }

    async fn get(&self, key: &str) -> Result<Value, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        let data = match key {
            "user:123" => {
                r#"{"id": 123, "name": "Alice", "email": "alice@example.com"}"#.as_bytes()
            }
            "user:456" => r#"{"id": 456, "name": "Bob", "email": "bob@example.com"}"#.as_bytes(),
            "session:abc" => b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
            "cache:config" => r#"{"theme": "dark", "lang": "en"}"#.as_bytes(),
            _ => return Err(BackendError::KeyNotFound(key.to_string())),
        };

        Ok(Value {
            data: data.to_vec(),
            value_type: if key.starts_with("user:") || key.starts_with("cache:") {
                ValueType::Json
            } else {
                ValueType::String
            },
            encoding: Some("utf8".to_string()),
        })
    }

    async fn get_many(&self, keys: &[String]) -> Result<Vec<(String, Value)>, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        let mut results = Vec::new();
        for key in keys {
            if let Ok(value) = self.get(key).await {
                results.push((key.clone(), value));
            }
        }
        Ok(results)
    }

    async fn set(
        &self,
        _key: &str,
        _value: &[u8],
        _ttl: Option<Duration>,
    ) -> Result<(), BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }
        if self.read_only {
            return Err(BackendError::ReadOnly);
        }
        Ok(())
    }

    async fn delete(&self, _key: &str) -> Result<bool, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }
        if self.read_only {
            return Err(BackendError::ReadOnly);
        }
        Ok(true)
    }

    async fn execute_raw(&self, command: &str) -> Result<RawCommandResult, BackendError> {
        if !self.connected {
            return Err(BackendError::ConnectionError("Not connected".to_string()));
        }

        Ok(RawCommandResult {
            output: format!("Mock response for: {}", command),
            status: CommandStatus::Success,
            duration: Duration::from_millis(1),
            error_message: None,
        })
    }
}
