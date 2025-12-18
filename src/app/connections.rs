use crate::backend::{Backend, BackendCapabilities};
use crate::types::ConnectionConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    connections: HashMap<String, Arc<RwLock<Box<dyn Backend>>>>,
    configs: HashMap<String, ConnectionConfig>,
    statuses: HashMap<String, ConnectionStatus>,
    capabilities: HashMap<String, BackendCapabilities>,
    active_id: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            configs: HashMap::new(),
            statuses: HashMap::new(),
            capabilities: HashMap::new(),
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

    /// Get all connection configs sorted by name for stable UI ordering
    pub fn get_configs(&self) -> Vec<&ConnectionConfig> {
        let mut configs: Vec<_> = self.configs.values().collect();
        configs.sort_by(|a, b| a.name.cmp(&b.name));
        configs
    }

    /// Get all connection configs as a Vec (for saving)
    pub fn get_all_configs(&self) -> Vec<ConnectionConfig> {
        let mut configs: Vec<_> = self.configs.values().cloned().collect();
        configs.sort_by(|a, b| a.name.cmp(&b.name));
        configs
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

    /// Set status for a connection
    pub fn set_status(&mut self, id: &str, status: ConnectionStatus) {
        self.statuses.insert(id.to_string(), status);
    }

    /// Set capabilities for a connection (mainly for tests)
    pub fn set_capabilities(&mut self, id: &str, caps: BackendCapabilities) {
        self.capabilities.insert(id.to_string(), caps);
    }

    /// Register an active, connected backend
    pub fn register_connection(
        &mut self,
        id: &str,
        backend: Arc<RwLock<Box<dyn Backend>>>,
        caps: BackendCapabilities,
    ) {
        self.connections.insert(id.to_string(), backend);
        self.capabilities.insert(id.to_string(), caps);
        self.active_id = Some(id.to_string());
        self.statuses
            .insert(id.to_string(), ConnectionStatus::Connected);
    }

    /// Disconnect from a backend
    pub async fn disconnect(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backend_arc) = self.connections.remove(id) {
            self.capabilities.remove(id);
            // We need to acquire write lock to disconnect
            let mut backend = backend_arc.write().await;
            backend.disconnect().await?;
        }

        self.statuses
            .insert(id.to_string(), ConnectionStatus::Disconnected);

        Ok(())
    }

    /// Get a handle to the active backend (thread-safe)
    pub fn get_active_backend_handle(&self) -> Option<Arc<RwLock<Box<dyn Backend>>>> {
        self.active_id
            .as_ref()
            .and_then(|id| self.connections.get(id))
            .cloned()
    }

    /// Get the active connection ID
    pub fn get_active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }

    /// Set the active connection
    pub fn set_active(&mut self, id: &str) -> bool {
        if self.configs.contains_key(id) {
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
        self.capabilities.remove(id);
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

    /// Get connection capabilities
    pub fn get_capabilities(&self, id: &str) -> Option<&BackendCapabilities> {
        self.capabilities.get(id)
    }

    /// Get active connection capabilities
    pub fn get_active_capabilities(&self) -> Option<&BackendCapabilities> {
        self.active_id
            .as_ref()
            .and_then(|id| self.get_capabilities(id))
    }

    /// Get all statuses
    pub fn get_statuses(&self) -> &HashMap<String, ConnectionStatus> {
        &self.statuses
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
