use super::{
    Backend, BackendCapabilities, BackendError, ConnectionInfo, RawCommandResult, ServerInfo,
};
use crate::types::{BackendType, ConnectionConfig, KeyMetadata, KeyScanResult, Value, ValueType};
use memcache::Client as MemcacheClient;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Debug, Clone)]
struct CachedumpEntry {
    key: String,
    expires_raw: Option<u64>,
}

/// Memcached backend implementation
pub struct MemcachedBackend {
    config: ConnectionConfig,
    client: Option<Arc<Mutex<MemcacheClient>>>,
    /// Cache of known keys (memcached doesn't have SCAN)
    key_cache: Arc<Mutex<HashSet<String>>>,
}

impl std::fmt::Debug for MemcachedBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemcachedBackend")
            .field("config", &self.config)
            .field("connected", &self.client.is_some())
            .finish()
    }
}

impl MemcachedBackend {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: None,
            key_cache: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Build Memcached connection URL from config
    fn build_connection_url(&self) -> String {
        // memcache crate uses format: "memcache://host:port"
        format!("memcache://{}:{}", self.config.host, self.config.port)
    }

    /// Get a reference to the client
    fn get_client(&self) -> Result<Arc<Mutex<MemcacheClient>>, BackendError> {
        self.client
            .clone()
            .ok_or_else(|| BackendError::ConnectionError("Not connected".to_string()))
    }

    /// Get list of slab IDs using stats slabs command
    async fn get_slab_ids(&self, host: &str, port: u16) -> Result<Vec<u32>, BackendError> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| BackendError::ConnectionError(e.to_string()))?;

        stream
            .write_all(b"stats slabs\r\n")
            .await
            .map_err(|e| BackendError::Internal(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut slab_ids = HashSet::new();
        let mut line = String::new();

        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            if line.trim() == "END" {
                break;
            }

            // Parse lines like: STAT 1:chunk_size 96
            if line.starts_with("STAT ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Some(slab_part) = parts[1].split(':').next() {
                        if let Ok(slab_id) = slab_part.parse::<u32>() {
                            slab_ids.insert(slab_id);
                        }
                    }
                }
            }
        }

        let mut sorted_slabs: Vec<u32> = slab_ids.into_iter().collect();
        sorted_slabs.sort_unstable();
        Ok(sorted_slabs)
    }

    /// Get keys from a specific slab using stats cachedump
    async fn get_keys_from_slab(
        &self,
        host: &str,
        port: u16,
        slab_id: u32,
        limit: usize,
    ) -> Result<Vec<CachedumpEntry>, BackendError> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| BackendError::ConnectionError(e.to_string()))?;

        let command = format!("stats cachedump {} {}\r\n", slab_id, limit);
        stream
            .write_all(command.as_bytes())
            .await
            .map_err(|e| BackendError::Internal(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut entries = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            if line.trim() == "END" {
                break;
            }

            // Parse lines like: ITEM test [5 b; 0 s]
            if line.starts_with("ITEM ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let expires_raw = line
                        .split(';')
                        .nth(1)
                        .and_then(|rest| rest.split_whitespace().next())
                        .and_then(|n| n.parse::<u64>().ok())
                        .and_then(|n| if n == 0 { None } else { Some(n) });
                    entries.push(CachedumpEntry {
                        key: parts[1].to_string(),
                        expires_raw,
                    });
                }
            }
        }

        Ok(entries)
    }

    async fn get_server_time_and_uptime(
        &self,
        host: &str,
        port: u16,
    ) -> Result<(Option<u64>, u64), BackendError> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| BackendError::ConnectionError(e.to_string()))?;

        stream
            .write_all(b"stats\r\n")
            .await
            .map_err(|e| BackendError::Internal(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let mut time_epoch = None;
        let mut uptime = 0u64;

        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            let trimmed = line.trim();
            if trimmed == "END" {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("STAT uptime ") {
                uptime = value.parse().unwrap_or(0);
            } else if let Some(value) = trimmed.strip_prefix("STAT time ") {
                time_epoch = value.parse().ok();
            }
        }

        Ok((time_epoch, uptime))
    }

    fn cachedump_ttl(
        now: SystemTime,
        server_time_epoch: Option<u64>,
        server_uptime: u64,
        expires_raw: u64,
    ) -> Option<Duration> {
        // The cachedump "expires" field is typically the item's `exptime` in seconds.
        // It is either:
        // - `0` (no expiration) - filtered before calling this
        // - a rel_time_t (seconds since server start when it will expire)
        // - a TTL delta in seconds (less common)
        // - or an absolute epoch timestamp on some setups
        if expires_raw >= 1_000_000_000 {
            let fallback_epoch = now.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs();
            let now_epoch = server_time_epoch.unwrap_or(fallback_epoch);
            return Some(Duration::from_secs(expires_raw.saturating_sub(now_epoch)));
        }

        if expires_raw > server_uptime {
            return Some(Duration::from_secs(expires_raw - server_uptime));
        }

        Some(Duration::from_secs(expires_raw))
    }
}

#[async_trait::async_trait]
impl Backend for MemcachedBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Memcached
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_ttl: true,
            supports_scan: true,          // Using stats cachedump
            supports_raw_commands: false, // Limited command support
            supports_batch_get: true,
            supports_efficient_pattern_search: false, // Memcached requires client-side filtering
            supports_type_display: false,             // Memcached is key-value only
            supports_expiry_display: true, // Memcached provides expiry via stats cachedump
            has_limited_key_listing: true,
        }
    }

    async fn connect(&mut self) -> Result<ConnectionInfo, BackendError> {
        let url = self.build_connection_url();

        // Check if auth is provided (not commonly supported by basic memcache clients)
        if self.config.auth.is_some() {
            return Err(BackendError::AuthenticationError(
                "This memcache client does not support SASL authentication".to_string(),
            ));
        }

        let host = self.config.host.clone();
        let port = self.config.port;

        // Connect in a blocking task
        let client = tokio::task::spawn_blocking(move || {
            MemcacheClient::connect(url).map_err(|e| BackendError::ConnectionError(e.to_string()))
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        // Test the connection with version command
        let client_arc = Arc::new(Mutex::new(client));
        let client_clone = client_arc.clone();

        let version = tokio::task::spawn_blocking(move || {
            let client = client_clone.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .version()
                .map_err(|e| BackendError::ConnectionError(e.to_string()))
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        let version_str = version
            .into_iter()
            .next()
            .map(|(_, v)| v)
            .unwrap_or_else(|| "unknown".to_string());

        self.client = Some(client_arc);

        Ok(ConnectionInfo {
            connected: true,
            server_version: version_str,
            address: format!("{}:{}", host, port),
        })
    }

    async fn disconnect(&mut self) -> Result<(), BackendError> {
        self.client = None;
        if let Ok(mut cache) = self.key_cache.lock() {
            cache.clear();
        }
        Ok(())
    }

    async fn ping(&self) -> Result<Duration, BackendError> {
        let client = self.get_client()?;
        let start = std::time::Instant::now();

        tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .version()
                .map_err(|e| BackendError::ConnectionError(e.to_string()))?;
            Ok::<_, BackendError>(())
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        Ok(start.elapsed())
    }

    async fn info(&self) -> Result<ServerInfo, BackendError> {
        let client = self.get_client()?;

        let stats = tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .stats()
                .map_err(|e| BackendError::Internal(e.to_string()))
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        // Parse stats from memcache
        let first_server_stats =
            stats.into_iter().next().map(|(_, s)| s).ok_or_else(|| {
                BackendError::Internal("No stats returned from server".to_string())
            })?;

        let mut version = String::from("unknown");
        let mut uptime = 0u64;
        let mut memory_used = 0u64;
        let mut memory_max = None;
        let mut clients_connected = 0usize;
        let mut keys_total = 0u64;

        for (key, value) in first_server_stats {
            match key.as_str() {
                "version" => version = value,
                "uptime" => uptime = value.parse().unwrap_or(0),
                "bytes" => memory_used = value.parse().unwrap_or(0),
                "limit_maxbytes" => {
                    let max: u64 = value.parse().unwrap_or(0);
                    if max > 0 {
                        memory_max = Some(max);
                    }
                }
                "curr_connections" => clients_connected = value.parse().unwrap_or(0),
                "curr_items" => keys_total = value.parse().unwrap_or(0),
                _ => {}
            }
        }

        Ok(ServerInfo {
            version,
            uptime: Duration::from_secs(uptime),
            memory_used,
            memory_max,
            clients_connected,
            keys_total,
        })
    }

    async fn scan_keys(
        &self,
        pattern: Option<&str>,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<KeyScanResult, BackendError> {
        // Use Memcached's stats cachedump to list keys
        let host = self.config.host.clone();
        let port = self.config.port;
        let now = SystemTime::now();
        let (server_time_epoch, server_uptime) =
            self.get_server_time_and_uptime(&host, port).await?;

        // Parse cursor to get current slab_id
        let current_slab: u32 = cursor.as_ref().and_then(|c| c.parse().ok()).unwrap_or(1);

        // Get all slab IDs first
        let slab_ids = self.get_slab_ids(&host, port).await?;

        if slab_ids.is_empty() {
            return Ok(KeyScanResult {
                keys: vec![],
                cursor: None,
                has_more: false,
            });
        }

        // Collect keys from current slab
        let mut all_keys: Vec<CachedumpEntry> = Vec::new();
        let mut next_slab = None;

        for &slab_id in slab_ids.iter() {
            if slab_id < current_slab {
                continue;
            }

            let keys = self
                .get_keys_from_slab(&host, port, slab_id, limit * 10)
                .await?;

            for key in keys {
                // Apply pattern filter
                let matches = if let Some(p) = pattern {
                    if p == "*" {
                        true
                    } else {
                        // Simple pattern matching
                        key.key.contains(p.trim_matches('*'))
                    }
                } else {
                    true
                };

                if matches {
                    all_keys.push(key);
                }

                if all_keys.len() >= limit {
                    break;
                }
            }

            if all_keys.len() >= limit {
                // Check if there are more slabs
                if let Some(&next_id) = slab_ids.iter().find(|&&id| id > slab_id) {
                    next_slab = Some(next_id);
                }
                break;
            }
        }

        // Get metadata for keys
        let mut key_metadata = Vec::new();
        for key in all_keys.into_iter().take(limit) {
            let mut metadata = match self.key_info(&key.key).await {
                Ok(metadata) => metadata,
                Err(_) => {
                    // Key might have been evicted, but we still want to show it.
                    // Create basic metadata without fetching the value.
                    KeyMetadata {
                        name: key.key.clone(),
                        value_type: ValueType::String,
                        size_bytes: 0,
                        ttl: None,
                        last_accessed: None,
                        encoding: None,
                        expires_at: None,
                    }
                }
            };

            if let Some(raw) = key.expires_raw {
                if let Some(ttl) = Self::cachedump_ttl(now, server_time_epoch, server_uptime, raw) {
                    metadata.ttl = Some(ttl);
                    metadata.expires_at = now.checked_add(ttl);
                }
            }

            key_metadata.push(metadata);
        }

        Ok(KeyScanResult {
            keys: key_metadata,
            cursor: next_slab.map(|id| id.to_string()),
            has_more: next_slab.is_some(),
        })
    }

    async fn search_keys(
        &self,
        pattern: &str,
        limit: usize,
    ) -> Result<KeyScanResult, BackendError> {
        // Memcached doesn't support efficient server-side pattern search
        // We need to scan all slabs and filter client-side
        let host = self.config.host.clone();
        let port = self.config.port;
        let now = SystemTime::now();
        let (server_time_epoch, server_uptime) =
            self.get_server_time_and_uptime(&host, port).await?;

        // Get all slab IDs
        let slab_ids = self.get_slab_ids(&host, port).await?;

        if slab_ids.is_empty() {
            return Ok(KeyScanResult {
                keys: vec![],
                cursor: None,
                has_more: false,
            });
        }

        // Prepare pattern for matching (strip wildcards for simple contains check)
        let search_term = pattern.trim_matches('*').to_lowercase();

        // Collect all matching keys from all slabs
        let mut matching_keys: Vec<CachedumpEntry> = Vec::new();

        for &slab_id in slab_ids.iter() {
            // Get a large batch of keys from each slab
            let keys = self.get_keys_from_slab(&host, port, slab_id, 1000).await?;

            for key in keys {
                // Client-side substring match (case-insensitive)
                if key.key.to_lowercase().contains(&search_term) {
                    matching_keys.push(key);
                    if matching_keys.len() >= limit {
                        break;
                    }
                }
            }

            if matching_keys.len() >= limit {
                break;
            }
        }

        // Truncate to limit
        matching_keys.truncate(limit);

        // Get metadata for matching keys
        let mut key_metadata = Vec::new();
        for key in matching_keys {
            let mut metadata = match self.key_info(&key.key).await {
                Ok(metadata) => metadata,
                Err(_) => KeyMetadata {
                    name: key.key.clone(),
                    value_type: ValueType::String,
                    size_bytes: 0,
                    ttl: None,
                    last_accessed: None,
                    encoding: None,
                    expires_at: None,
                },
            };

            if let Some(raw) = key.expires_raw {
                if let Some(ttl) = Self::cachedump_ttl(now, server_time_epoch, server_uptime, raw) {
                    metadata.ttl = Some(ttl);
                    metadata.expires_at = now.checked_add(ttl);
                }
            }

            key_metadata.push(metadata);
        }

        Ok(KeyScanResult {
            keys: key_metadata,
            cursor: None,
            has_more: false,
        })
    }

    async fn key_count(&self, _pattern: Option<&str>) -> Result<u64, BackendError> {
        // Get from stats
        let info = self.info().await?;
        Ok(info.keys_total)
    }

    async fn key_info(&self, key: &str) -> Result<KeyMetadata, BackendError> {
        let client = self.get_client()?;
        let key_str = key.to_string();
        let key_cache = self.key_cache.clone();

        let data: Option<Vec<u8>> = tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .get(&key_str)
                .map_err(|e| BackendError::Internal(e.to_string()))
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        if let Some(data) = data {
            let size_bytes = data.len() as u64;

            // Add to key cache
            if let Ok(mut cache) = key_cache.lock() {
                cache.insert(key.to_string());
            }

            // Determine value type
            let value_type = if super::utils::is_json(&data) {
                ValueType::Json
            } else if data.iter().all(|&b| b.is_ascii()) {
                ValueType::String
            } else {
                ValueType::Binary
            };

            Ok(KeyMetadata {
                name: key.to_string(),
                value_type,
                size_bytes,
                ttl: None, // Memcached doesn't expose TTL information
                last_accessed: Some(SystemTime::now()),
                encoding: Some("utf8".to_string()),
                expires_at: None,
            })
        } else {
            Err(BackendError::KeyNotFound(key.to_string()))
        }
    }

    async fn get(&self, key: &str) -> Result<Value, BackendError> {
        let client = self.get_client()?;
        let key_str = key.to_string();
        let key_cache = self.key_cache.clone();

        let data: Option<Vec<u8>> = tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .get(&key_str)
                .map_err(|e| BackendError::Internal(e.to_string()))
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        if let Some(data) = data {
            // Add to key cache
            if let Ok(mut cache) = key_cache.lock() {
                cache.insert(key.to_string());
            }

            // Detect value type
            let value_type = if super::utils::is_json(&data) {
                ValueType::Json
            } else if data.iter().all(|&b| b.is_ascii()) {
                ValueType::String
            } else {
                ValueType::Binary
            };

            Ok(Value {
                data,
                value_type,
                encoding: Some("utf8".to_string()),
            })
        } else {
            Err(BackendError::KeyNotFound(key.to_string()))
        }
    }

    async fn get_many(&self, keys: &[String]) -> Result<Vec<(String, Value)>, BackendError> {
        let client = self.get_client()?;
        let keys = keys.to_vec();
        let key_cache = self.key_cache.clone();

        let results: std::collections::HashMap<String, Vec<u8>> =
            tokio::task::spawn_blocking(move || {
                let client = client.lock().map_err(|e| {
                    BackendError::Internal(format!(
                        "Mutex poisoned (a thread panicked while holding the lock): {}",
                        e
                    ))
                })?;

                let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
                client
                    .gets(&key_refs)
                    .map_err(|e| BackendError::Internal(e.to_string()))
            })
            .await
            .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        // Add all keys to cache
        if let Ok(mut cache) = key_cache.lock() {
            for key in results.keys() {
                cache.insert(key.clone());
            }
        }

        let mut values = Vec::new();
        for (key, data) in results {
            let value_type = if super::utils::is_json(&data) {
                ValueType::Json
            } else if data.iter().all(|&b| b.is_ascii()) {
                ValueType::String
            } else {
                ValueType::Binary
            };

            values.push((
                key,
                Value {
                    data,
                    value_type,
                    encoding: Some("utf8".to_string()),
                },
            ));
        }

        Ok(values)
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

        let client = self.get_client()?;
        let key = key.to_string();
        let value = value.to_vec();
        let key_cache = self.key_cache.clone();

        let ttl_secs = ttl.map(|d| d.as_secs() as u32).unwrap_or(0);

        tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            client
                .set(&key, &value[..], ttl_secs)
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            // Add to key cache
            if let Ok(mut cache) = key_cache.lock() {
                cache.insert(key);
            }

            Ok::<_, BackendError>(())
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, BackendError> {
        if self.config.read_only {
            return Err(BackendError::ReadOnly);
        }

        let client = self.get_client()?;
        let key = key.to_string();
        let key_cache = self.key_cache.clone();

        let deleted = tokio::task::spawn_blocking(move || {
            let client = client.lock().map_err(|e| {
                BackendError::Internal(format!(
                    "Mutex poisoned (a thread panicked while holding the lock): {}",
                    e
                ))
            })?;
            let result = client
                .delete(&key)
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            // Remove from key cache
            if let Ok(mut cache) = key_cache.lock() {
                cache.remove(&key);
            }

            Ok::<bool, BackendError>(result)
        })
        .await
        .map_err(|e| BackendError::Internal(format!("Join error: {}", e)))??;

        Ok(deleted)
    }

    async fn execute_raw(&self, _command: &str) -> Result<RawCommandResult, BackendError> {
        // Memcached doesn't support arbitrary raw commands through this client
        Err(BackendError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test".to_string(),
            name: "Test Memcached".to_string(),
            backend_type: BackendType::Memcached,
            host: "localhost".to_string(),
            port: 11211,
            auth: None,
            database: None,
            tls: None,
            timeout: Duration::from_secs(5),
            read_only: false,
        }
    }

    #[test]
    fn test_build_connection_url_basic() {
        let backend = MemcachedBackend::new(create_test_config());
        let url = backend.build_connection_url();
        assert_eq!(url, "memcache://localhost:11211");
    }

    #[test]
    fn test_build_connection_url_custom_port() {
        let mut config = create_test_config();
        config.port = 11212;
        let backend = MemcachedBackend::new(config);
        let url = backend.build_connection_url();
        assert_eq!(url, "memcache://localhost:11212");
    }

    #[test]
    fn test_backend_type() {
        let backend = MemcachedBackend::new(create_test_config());
        assert_eq!(backend.backend_type(), BackendType::Memcached);
    }

    #[test]
    fn test_capabilities() {
        let backend = MemcachedBackend::new(create_test_config());
        let caps = backend.capabilities();
        assert!(caps.supports_ttl);
        assert!(caps.supports_scan); // Using stats cachedump
        assert!(!caps.supports_raw_commands);
        assert!(caps.supports_batch_get);
        assert!(!caps.supports_efficient_pattern_search); // Client-side only
    }

    #[test]
    fn test_get_client_not_connected() {
        let backend = MemcachedBackend::new(create_test_config());
        let result = backend.get_client();
        assert!(matches!(result, Err(BackendError::ConnectionError(_))));
    }
}
