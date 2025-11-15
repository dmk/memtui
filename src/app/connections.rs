use crate::backend::{Backend, MockBackend, RedisBackend};
use crate::types::{BackendType, ConnectionConfig};
use std::collections::HashMap;
use std::time::Duration;

/// Manages multiple backend connections
pub struct ConnectionManager {
    connections: HashMap<String, Box<dyn Backend>>,
    configs: HashMap<String, ConnectionConfig>,
    active_id: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let mut manager = Self {
            connections: HashMap::new(),
            configs: HashMap::new(),
            active_id: None,
        };

        // Add default mock connection configuration
        let mock_config = ConnectionConfig {
            id: "mock".to_string(),
            name: "Mock Backend".to_string(),
            backend_type: BackendType::Redis,
            host: "localhost".to_string(),
            port: 6379,
            auth: None,
            database: Some("0".to_string()),
            tls: None,
            timeout: Duration::from_secs(2),
            read_only: true,
        };

        manager.configs.insert("mock".to_string(), mock_config);
        manager
    }

    /// Add a new connection configuration
    pub fn add_connection(&mut self, config: ConnectionConfig) {
        let id = config.id.clone();
        self.configs.insert(id, config);
    }

    /// Get all connection configs
    pub fn get_configs(&self) -> Vec<&ConnectionConfig> {
        self.configs.values().collect()
    }

    /// Get a connection config by ID
    pub fn get_config(&self, id: &str) -> Option<&ConnectionConfig> {
        self.configs.get(id)
    }

    /// Connect to a backend
    pub async fn connect(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.connections.contains_key(id) {
            self.active_id = Some(id.to_string());
            return Ok(());
        }

        let config = self.configs.get(id).ok_or("Connection not found")?.clone();

        // Special-case: mock backend uses in-memory data
        if id == "mock" {
            let mut backend: Box<dyn Backend> = Box::new(MockBackend::new(config.read_only));
            backend.connect().await?;
            self.connections.insert(id.to_string(), backend);
            self.active_id = Some(id.to_string());
            return Ok(());
        }

        // Create backend based on type
        let mut backend: Box<dyn Backend> = match config.backend_type {
            BackendType::Redis => Box::new(RedisBackend::new(config)),
            BackendType::Memcached => {
                return Err("Memcached not yet implemented".into());
            }
            BackendType::Etcd => {
                return Err("Etcd not yet implemented".into());
            }
        };

        // Attempt connection
        backend.connect().await?;

        // Store the connection
        self.connections.insert(id.to_string(), backend);
        self.active_id = Some(id.to_string());

        Ok(())
    }

    /// Disconnect from a backend
    pub async fn disconnect(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut backend) = self.connections.remove(id) {
            backend.disconnect().await?;
        }

        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }

        Ok(())
    }

    /// Get the active backend
    pub fn get_active_backend(&self) -> Option<&dyn Backend> {
        self.active_id
            .as_ref()
            .and_then(|id| self.connections.get(id))
            .map(|b| b.as_ref())
    }

    /// Get the active backend mutably
    pub fn get_active_backend_mut(&mut self) -> Option<&mut Box<dyn Backend>> {
        self.active_id
            .as_ref()
            .and_then(|id| self.connections.get_mut(id))
    }

    /// Get the active connection ID
    pub fn get_active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }

    /// Set the active connection
    pub fn set_active(&mut self, id: &str) -> bool {
        if self.connections.contains_key(id) {
            self.active_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Check if a connection is active (connected)
    pub fn is_connected(&self, id: &str) -> bool {
        self.connections.contains_key(id)
    }

    /// Remove a connection configuration
    pub fn remove_config(&mut self, id: &str) {
        self.configs.remove(id);
        self.connections.remove(id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
