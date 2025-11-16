use crate::backend::{Backend, MemcachedBackend, RedisBackend};
use crate::types::{BackendType, ConnectionConfig};
use std::collections::HashMap;

/// Connection status for a backend
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Manages multiple backend connections
pub struct ConnectionManager {
    connections: HashMap<String, Box<dyn Backend>>,
    configs: HashMap<String, ConnectionConfig>,
    statuses: HashMap<String, ConnectionStatus>,
    active_id: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            configs: HashMap::new(),
            statuses: HashMap::new(),
            active_id: None,
        }
    }

    /// Add a new connection configuration
    pub fn add_connection(&mut self, config: ConnectionConfig) {
        let id = config.id.clone();
        self.statuses
            .insert(id.clone(), ConnectionStatus::Disconnected);
        self.configs.insert(id, config);
    }

    /// Get all connection configs
    pub fn get_configs(&self) -> Vec<&ConnectionConfig> {
        self.configs.values().collect()
    }

    /// Get all connection configs as a Vec (for saving)
    pub fn get_all_configs(&self) -> Vec<ConnectionConfig> {
        self.configs.values().cloned().collect()
    }

    /// Load connections from a list
    pub fn load_configs(&mut self, configs: Vec<ConnectionConfig>) {
        self.configs.clear();
        self.statuses.clear();
        for config in configs {
            let id = config.id.clone();
            self.statuses
                .insert(id.clone(), ConnectionStatus::Disconnected);
            self.configs.insert(id, config);
        }
    }

    /// Get a connection config by ID
    pub fn get_config(&self, id: &str) -> Option<&ConnectionConfig> {
        self.configs.get(id)
    }

    /// Connect to a backend with timeout
    pub async fn connect(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.connections.contains_key(id) {
            self.active_id = Some(id.to_string());
            self.statuses
                .insert(id.to_string(), ConnectionStatus::Connected);
            return Ok(());
        }

        self.statuses
            .insert(id.to_string(), ConnectionStatus::Connecting);

        let config = self.configs.get(id).ok_or("Connection not found")?.clone();

        // Create backend based on type
        let mut backend: Box<dyn Backend> = match config.backend_type {
            BackendType::Redis => Box::new(RedisBackend::new(config)),
            BackendType::Memcached => Box::new(MemcachedBackend::new(config)),
            BackendType::Etcd => {
                self.statuses.insert(
                    id.to_string(),
                    ConnectionStatus::Error("Etcd not yet implemented".to_string()),
                );
                return Err("Etcd not yet implemented".into());
            }
        };

        // Attempt connection with timeout
        let connect_result =
            tokio::time::timeout(std::time::Duration::from_secs(5), backend.connect()).await;

        match connect_result {
            Ok(Ok(_)) => {
                // Store the connection
                self.connections.insert(id.to_string(), backend);
                self.active_id = Some(id.to_string());
                self.statuses
                    .insert(id.to_string(), ConnectionStatus::Connected);
                Ok(())
            }
            Ok(Err(e)) => {
                let err_msg = e.to_string();
                self.statuses
                    .insert(id.to_string(), ConnectionStatus::Error(err_msg.clone()));
                Err(Box::new(e))
            }
            Err(_) => {
                let err_msg = "Connection timeout".to_string();
                self.statuses
                    .insert(id.to_string(), ConnectionStatus::Error(err_msg.clone()));
                Err(err_msg.into())
            }
        }
    }

    /// Disconnect from a backend
    pub async fn disconnect(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut backend) = self.connections.remove(id) {
            backend.disconnect().await?;
        }

        self.statuses
            .insert(id.to_string(), ConnectionStatus::Disconnected);

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
        self.statuses.remove(id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
    }

    /// Get connection status
    pub fn get_status(&self, id: &str) -> ConnectionStatus {
        self.statuses
            .get(id)
            .cloned()
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    /// Get the status of the active connection
    pub fn get_active_status(&self) -> Option<ConnectionStatus> {
        self.active_id.as_ref().map(|id| self.get_status(id))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
