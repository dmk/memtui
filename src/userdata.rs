use crate::types::ConnectionConfig;
use std::fs;
use std::io;
use std::path::PathBuf;

const MAX_RECENT_CONNECTIONS: usize = 8;

/// Get the path to the userdata directory
pub fn get_userdata_dir() -> Result<PathBuf, io::Error> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;

    let userdata_dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("memtui");

    // Create directory if it doesn't exist
    if !userdata_dir.exists() {
        fs::create_dir_all(&userdata_dir)?;
    }

    Ok(userdata_dir)
}

/// Get the path to the connections file
pub fn get_connections_file() -> Result<PathBuf, io::Error> {
    Ok(get_userdata_dir()?.join("connections.json"))
}

/// Load connections from userdata
pub fn load_connections() -> Result<Vec<ConnectionConfig>, Box<dyn std::error::Error>> {
    let file_path = get_connections_file()?;

    if !file_path.exists() {
        // No file exists yet, return empty list
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(file_path)?;
    let connections: Vec<ConnectionConfig> = serde_json::from_str(&contents)?;

    Ok(connections)
}

/// Save connections to userdata
pub fn save_connections(
    connections: &[ConnectionConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_connections_file()?;
    let json = serde_json::to_string_pretty(connections)?;
    fs::write(file_path, json)?;

    Ok(())
}

fn get_recent_connections_file() -> Result<PathBuf, io::Error> {
    Ok(get_userdata_dir()?.join("recent_connections.json"))
}

/// Load the ordered list of recently used connection IDs
pub fn load_recent_connection_ids() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file_path = get_recent_connections_file()?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(file_path)?;
    let ids: Vec<String> = serde_json::from_str(&contents)?;
    Ok(ids)
}

pub fn save_recent_connection_ids(ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_recent_connections_file()?;
    let json = serde_json::to_string_pretty(ids)?;
    fs::write(file_path, json)?;
    Ok(())
}

/// Record a connection ID as the most recently used and return the updated list
pub fn record_recent_connection_id(id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = load_recent_connection_ids().unwrap_or_default();
    ids.retain(|existing| existing != id);
    ids.insert(0, id.to_string());
    if ids.len() > MAX_RECENT_CONNECTIONS {
        ids.truncate(MAX_RECENT_CONNECTIONS);
    }
    save_recent_connection_ids(&ids)?;
    Ok(ids)
}

/// Remove a connection ID from recents (used when deleting configs)
pub fn remove_recent_connection_id(id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = load_recent_connection_ids().unwrap_or_default();
    let len_before = ids.len();
    ids.retain(|existing| existing != id);
    if ids.len() != len_before {
        save_recent_connection_ids(&ids)?;
    }
    Ok(ids)
}
