mod connections;

pub use connections::{ConnectionManager, ConnectionStatus};

use crate::formatter::{JsonColorConfig, JsonFormatter, TextFormatter};
use crate::types::{KeyMetadata, Value};

/// Main application state (backend connection + data)
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub keys: Vec<Option<KeyMetadata>>, // Sparse array - None = not loaded yet
    pub selected_value: Option<Value>,
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
        self.selected_value = None;
        self.reset_pagination();
        self.connection_manager.disconnect(id).await
    }

    /// Reset pagination state
    fn reset_pagination(&mut self) {
        self.keys_cursor = None;
        self.has_more_keys = false;
        self.total_key_count = None;
        self.is_loading_keys = false;
    }

    pub async fn load_keys(&mut self) {
        self.reset_pagination();
        self.is_loading_keys = true;

        if let Some(backend) = self.connection_manager.get_active_backend() {
            // First, get the total count
            if let Ok(count) = backend.key_count(None).await {
                self.total_key_count = Some(count);
                // Initialize sparse array with placeholders
                self.keys = vec![None; count as usize];
            }

            // Load 3N rows initially (prev N + current N + next N)
            let initial_load_count = self.viewport_height * 3;

            match backend.scan_keys(None, None, initial_load_count).await {
                Ok(result) => {
                    // Fill in the first 3N rows
                    for (i, key) in result.keys.into_iter().enumerate() {
                        if i < self.keys.len() {
                            self.keys[i] = Some(key);
                        }
                    }
                    self.keys_cursor = result.cursor;
                    self.has_more_keys = result.has_more;
                    self.error_message = None;
                }
                Err(e) => {
                    self.keys.clear();
                    self.error_message = Some(format!("Failed to load keys: {}", e));
                }
            }
        }

        self.is_loading_keys = false;

        // After initial load, check if we need wrap-around (we start at position 0)
        if self.needs_loading_around(0) {
            self.load_more_keys_for_center(0).await;
        }
    }

    /// Load more keys prioritizing the region around `center` (wrap-aware).
    /// Loads 2N rows at a time (N above + N below viewport) into empty slots near the center.
    pub async fn load_more_keys_for_center(&mut self, center: usize) {
        if !self.has_more_keys || self.is_loading_keys {
            return;
        }

        self.is_loading_keys = true;

        // Load 2N rows to cover viewport above and below
        let chunk_size = self.viewport_height * 2;

        // Compute preferred indices to fill (wrap-aware)
        let total = self.keys.len();
        let n = self.viewport_height;
        let mut preferred_indices: Vec<usize> = Vec::with_capacity(chunk_size);
        if total > 0 {
            let start = center.saturating_sub(n);
            let end = (center + n).min(total);
            // main window
            for i in start..end {
                if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                    preferred_indices.push(i);
                    if preferred_indices.len() >= chunk_size {
                        break;
                    }
                }
            }
            // wrap end segment if near start
            if preferred_indices.len() < chunk_size && center < n {
                let wrap_start = total.saturating_sub(n);
                for i in wrap_start..total {
                    if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                        preferred_indices.push(i);
                        if preferred_indices.len() >= chunk_size {
                            break;
                        }
                    }
                }
            }
            // wrap start segment if near end
            if preferred_indices.len() < chunk_size && center + n >= total {
                for i in 0..n.min(total) {
                    if self.keys.get(i).and_then(|k| k.as_ref()).is_none() {
                        preferred_indices.push(i);
                        if preferred_indices.len() >= chunk_size {
                            break;
                        }
                    }
                }
            }
        }

        if let Some(backend) = self.connection_manager.get_active_backend() {
            match backend
                .scan_keys(None, self.keys_cursor.clone(), chunk_size)
                .await
            {
                Ok(result) => {
                    // Fill preferred empty slots around the center first
                    let mut keys_iter = result.keys.into_iter();
                    for idx in preferred_indices {
                        if let Some(key) = keys_iter.next() {
                            if self.keys.get(idx).is_some() && self.keys[idx].is_none() {
                                self.keys[idx] = Some(key);
                            }
                        } else {
                            break;
                        }
                    }
                    // If there are still keys left, place them into any remaining empty slots
                    for key in keys_iter {
                        if let Some(empty_idx) = self.keys.iter().position(|k| k.is_none()) {
                            self.keys[empty_idx] = Some(key);
                        } else {
                            break;
                        }
                    }

                    self.keys_cursor = result.cursor;
                    self.has_more_keys = result.has_more;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load more keys: {}", e));
                }
            }
        }

        self.is_loading_keys = false;
    }

    /// Keep scanning until the region around `index` is filled or we run out of data.
    /// Safety cap via `max_iterations` to avoid long loops.
    pub async fn load_until_filled_around(&mut self, index: usize, max_iterations: usize) {
        let mut iterations = 0;
        while self.has_more_keys && self.needs_loading_around(index) && iterations < max_iterations
        {
            self.load_more_keys_for_center(index).await;
            iterations += 1;
        }
    }

    /// Check if we need to load the region around a specific index
    /// Uses viewport size N - checks N rows above and N rows below (2N total)
    /// Also handles wrap-around: when near start, checks end; when near end, checks start
    pub fn needs_loading_around(&self, index: usize) -> bool {
        if self.is_loading_keys || !self.has_more_keys {
            return false;
        }

        let n = self.viewport_height;
        let total = self.keys.len();

        if total == 0 {
            return false;
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

    /// Get the range that needs loading around a position
    pub fn get_loading_range(&self, index: usize) -> (usize, usize) {
        let n = self.viewport_height;
        let start = index.saturating_sub(n);
        let end = (index + n).min(self.keys.len());
        (start, end)
    }

    pub async fn update_value(&mut self, selected_index: Option<usize>) {
        if let Some(i) = selected_index
            && let Some(Some(key)) = self.keys.get(i)  // Handle Option<Option<KeyMetadata>>
            && let Some(backend) = self.connection_manager.get_active_backend()
        {
            match backend.get(&key.name).await {
                Ok(value) => {
                    self.selected_value = Some(value);
                    self.error_message = None;
                }
                Err(e) => {
                    self.selected_value = None;
                    self.error_message = Some(format!("Error loading value: {}", e));
                }
            }
        } else if selected_index.is_some() {
            // Selected an unloaded placeholder
            self.selected_value = None;
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
