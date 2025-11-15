use crate::backend::Backend;
use crate::formatter::{Formatter, TextFormatter};
use crate::types::KeyMetadata;

/// Main application state (backend connection + data)
pub struct AppState {
    pub backend: Box<dyn Backend>,
    pub keys: Vec<KeyMetadata>,
    pub selected_value: String,
    pub formatter: TextFormatter,
}

impl AppState {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            keys: Vec::new(),
            selected_value: String::new(),
            formatter: TextFormatter,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.connect().await?;
        Ok(())
    }

    pub async fn load_keys(&mut self) {
        match self.backend.scan_keys(None, None, 100).await {
            Ok(result) => {
                self.keys = result.keys;
            }
            Err(_) => {
                self.keys.clear();
            }
        }
    }

    pub async fn update_value(&mut self, selected_index: Option<usize>) {
        if let Some(i) = selected_index
            && let Some(key) = self.keys.get(i)
        {
            match self.backend.get(&key.name).await {
                Ok(value) => {
                    self.selected_value = self
                        .formatter
                        .format(&value)
                        .unwrap_or_else(|_| "<formatting error>".to_string());
                }
                Err(_) => {
                    self.selected_value = "<error loading value>".to_string();
                }
            }
        }
    }
}
