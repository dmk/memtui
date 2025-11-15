use crate::types::ConnectionConfig;
use std::fs;
use std::io;
use std::path::PathBuf;

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
