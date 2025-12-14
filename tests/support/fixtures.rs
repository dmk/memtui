#![allow(dead_code)]

use std::time::{Duration, SystemTime};

use memtui::app::ConnectionStatus;
use memtui::types::{BackendType, ConnectionConfig, KeyMetadata, ValueType};

/// Build a simple connection config for tests.
#[allow(dead_code)]
pub fn connection(id: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: id.to_string(),
        name: id.to_string(),
        backend_type: BackendType::Redis,
        host: "127.0.0.1".to_string(),
        port: 6379,
        auth: None,
        database: None,
        tls: None,
        timeout: Duration::from_secs(5),
        read_only: true,
    }
}

/// Register a connected, active connection on the app state.
#[allow(dead_code)]
pub fn attach_connected(app_state: &mut memtui::app::AppState, id: &str) {
    let cfg = connection(id);
    app_state.connection_manager.add_connection(cfg);
    app_state
        .connection_manager
        .set_status(id, ConnectionStatus::Connected);
    app_state.connection_manager.set_active(id);
}

/// Common set of keys for rendering tests.
#[allow(dead_code)]
pub fn sample_keys() -> Vec<KeyMetadata> {
    vec![
        KeyMetadata {
            name: "user:123".to_string(),
            value_type: ValueType::Json,
            size_bytes: 85,
            ttl: Some(Duration::from_secs(3600)),
            last_accessed: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            encoding: Some("utf8".to_string()),
        },
        KeyMetadata {
            name: "user:456".to_string(),
            value_type: ValueType::Json,
            size_bytes: 92,
            ttl: Some(Duration::from_secs(3600)),
            last_accessed: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2)),
            encoding: Some("utf8".to_string()),
        },
        KeyMetadata {
            name: "session:abc".to_string(),
            value_type: ValueType::String,
            size_bytes: 128,
            ttl: Some(Duration::from_secs(1800)),
            last_accessed: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(3)),
            encoding: Some("utf8".to_string()),
        },
    ]
}

/// Fill the app state's key storage with the provided list.
#[allow(dead_code)]
pub fn set_keys(app_state: &mut memtui::app::AppState, keys: Vec<KeyMetadata>) {
    app_state.keys = keys.into_iter().map(Some).collect();
    app_state.total_key_count = Some(app_state.keys.len() as u64);
    app_state.has_more_keys = false;
}
