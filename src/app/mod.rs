mod connections;

pub use connections::ConnectionManager;

use crate::formatter::{Formatter, TextFormatter};
use crate::types::KeyMetadata;

/// Main application state (backend connection + data)
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub keys: Vec<KeyMetadata>,
    pub selected_value: String,
    pub formatter: TextFormatter,
    pub error_message: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection_manager: ConnectionManager::new(),
            keys: Vec::new(),
            selected_value: String::new(),
            formatter: TextFormatter,
            error_message: None,
        }
    }

    pub async fn connect_to(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.error_message = None;
        match self.connection_manager.connect(id).await {
            Ok(_) => {
                self.load_keys().await;
                Ok(())
            }
            Err(e) => {
                self.error_message = Some(format!("Connection failed: {}", e));
                Err(e)
            }
        }
    }

    pub async fn disconnect_from(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.error_message = None;
        self.keys.clear();
        self.selected_value.clear();
        self.connection_manager.disconnect(id).await
    }

    pub async fn load_keys(&mut self) {
        if let Some(backend) = self.connection_manager.get_active_backend() {
            match backend.scan_keys(None, None, 100).await {
                Ok(result) => {
                    self.keys = result.keys;
                    self.error_message = None;
                }
                Err(e) => {
                    self.keys.clear();
                    self.error_message = Some(format!("Failed to load keys: {}", e));
                }
            }
        }
    }

    pub async fn update_value(&mut self, selected_index: Option<usize>) {
        if let Some(i) = selected_index
            && let Some(key) = self.keys.get(i)
            && let Some(backend) = self.connection_manager.get_active_backend()
        {
            match backend.get(&key.name).await {
                Ok(value) => {
                    self.selected_value = self
                        .formatter
                        .format(&value)
                        .unwrap_or_else(|_| "<formatting error>".to_string());
                    self.error_message = None;
                }
                Err(e) => {
                    self.selected_value = format!("<error loading value: {}>", e);
                }
            }
        }
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
