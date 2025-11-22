mod connections;

pub use connections::{ConnectionManager, ConnectionStatus};

use crate::formatter::{JsonColorConfig, JsonFormatter, TextFormatter};
use crate::types::{KeyMetadata, Value};

/// Main application state (backend connection + data)
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub keys: Vec<Option<KeyMetadata>>, // Sparse array - None = not loaded yet
    pub selected_value: Option<Value>,
    pub selected_key_index: Option<usize>,
    pub value_request_token: u64,
    pub text_formatter: TextFormatter,
    pub json_formatter: JsonFormatter,
    pub error_message: Option<String>,
    // Pagination state
    pub keys_cursor: Option<String>,
    pub has_more_keys: bool,
    pub total_key_count: Option<u64>,
    pub is_loading_keys: bool,
    pub keys_per_chunk: usize,
    pub viewport_height: usize, // Number of rows visible at once
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection_manager: ConnectionManager::new(),
            keys: Vec::new(),
            selected_value: None,
            selected_key_index: None,
            value_request_token: 0,
            text_formatter: TextFormatter,
            json_formatter: JsonFormatter::new(JsonColorConfig::default()),
            error_message: None,
            keys_cursor: None,
            has_more_keys: false,
            total_key_count: None,
            is_loading_keys: false,
            keys_per_chunk: 200, // Reasonable chunk size for lazy loading
            viewport_height: 20, // Will be updated based on actual terminal size
        }
    }

    /// Update JSON color configuration
    pub fn set_json_config(&mut self, config: JsonColorConfig) {
        self.json_formatter = JsonFormatter::new(config);
    }

    pub fn reset_pagination(&mut self) {
        self.keys_cursor = None;
        self.has_more_keys = false;
        self.total_key_count = None;
        self.is_loading_keys = false;
        self.keys.clear();
        self.selected_value = None;
        self.selected_key_index = None;
    }

    /// Calculate which indices in the sparse array are empty and should be filled
    /// prioritizing the region around `center`.
    /// Returns a list of indices that need filling.
    pub fn get_preferred_indices_for_filling(&self, center: usize, count: usize) -> Vec<usize> {
        let total = self.keys.len();
        let n = self.viewport_height;
        let mut preferred_indices: Vec<usize> = Vec::with_capacity(count);

        if total > 0 {
            let start = center.saturating_sub(n);
            let end = (center + n).min(total);
            // main window
            for i in start..end {
                if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    preferred_indices.push(i);
                    if preferred_indices.len() >= count {
                        return preferred_indices;
                    }
                }
            }
            // wrap end segment if near start
            if preferred_indices.len() < count && center < n {
                let wrap_start = total.saturating_sub(n);
                for i in wrap_start..total {
                    if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                        preferred_indices.push(i);
                        if preferred_indices.len() >= count {
                            return preferred_indices;
                        }
                    }
                }
            }
            // wrap start segment if near end
            if preferred_indices.len() < count && center + n >= total {
                for i in 0..n.min(total) {
                    if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                        preferred_indices.push(i);
                        if preferred_indices.len() >= count {
                            return preferred_indices;
                        }
                    }
                }
            }
        }
        preferred_indices
    }

    /// Check if we need to load the region around a specific index
    /// Uses viewport size N - checks N rows above and N rows below (2N total)
    /// Also handles wrap-around: when near start, checks end; when near end, checks start
    pub fn needs_loading_around(&self, index: usize) -> bool {
        if self.is_loading_keys || (!self.has_more_keys && self.total_key_count.is_some()) {
            // If fully loaded or currently loading, don't need more
            if !self.has_more_keys && self.keys.iter().all(|k| k.is_some()) {
                return false;
            }
            // If we have gaps but no more keys to fetch from backend (cursor exhausted), we can't load more.
            if !self.has_more_keys {
                return false;
            }
        }
        // ... actually simplistic check: if we are loading, return false to avoid duplicate requests
        if self.is_loading_keys {
            return false;
        }

        let n = self.viewport_height;
        let total = self.keys.len();

        if total == 0 {
            // If we haven't initialized keys yet (total count unknown or 0), we might need loading if cursor is None (initial load)
            // But usually total_key_count is set before keys.
            return self.total_key_count.is_none();
        }

        // Check N rows above and N rows below current position
        let start = index.saturating_sub(n);
        let end = (index + n).min(total);

        for i in start..end {
            if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                return true;
            }
        }

        // Wrap-around: if near start, check last N rows (for wrapping up)
        if index < n {
            let wrap_start = total.saturating_sub(n);
            for i in wrap_start..total {
                if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        // Wrap-around: if near end, check first N rows (for wrapping down)
        if index + n >= total {
            for i in 0..n.min(total) {
                if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    return true;
                }
            }
        }

        false
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
