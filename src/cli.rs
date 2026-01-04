use crate::types::{Auth, BackendType, ConnectionConfig};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

/// memtui - Interactive TUI for browsing key-value stores
#[derive(Parser, Debug, Clone)]
#[command(name = "memtui")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to log file (logging disabled if not specified)
    #[arg(short, long, value_name = "FILE")]
    pub log_file: Option<PathBuf>,

    /// Log level
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    pub log_level: LogLevel,

    /// Connect directly by saved connection name
    #[arg(short = 'C', long, value_name = "NAME")]
    pub connect: Option<String>,

    /// Connection string for one-shot connection (not saved)
    /// Format: redis://[user:pass@]host:port[/db]
    ///         memcached://host:port
    ///         etcd://host:port
    #[arg(value_name = "CONNECTION_STRING")]
    pub connection_string: Option<String>,

    /// Enable debug mode (F12 to freeze UI and inspect)
    #[arg(long, env = "MEMTUI_DEBUG")]
    pub debug: bool,

    /// Enable optional features (comma-separated or multiple --with flags)
    /// Available: advanced-cmd (enables :command, g goto, >raw modes)
    #[arg(long = "with", value_name = "FEATURE", value_delimiter = ',')]
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Feature {
    /// Enable advanced cmdline modes (:command, g goto, >raw)
    #[value(name = "advanced-cmd")]
    AdvancedCmd,
}

impl Cli {
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Represents a parsed connection string
#[derive(Debug, Clone)]
pub struct ParsedConnection {
    pub backend_type: BackendType,
    pub host: String,
    pub port: u16,
    pub auth: Option<Auth>,
    pub database: Option<String>,
}

impl ParsedConnection {
    /// Convert to a ConnectionConfig with a temporary ID
    pub fn to_config(&self, timeout: Duration) -> ConnectionConfig {
        let display_name = format!(
            "{}://{}:{}",
            match self.backend_type {
                BackendType::Redis => "redis",
                BackendType::Memcached => "memcached",
                BackendType::Etcd => "etcd",
            },
            self.host,
            self.port
        );

        ConnectionConfig {
            id: format!("__temp__{}", uuid_simple()),
            name: display_name,
            backend_type: self.backend_type,
            host: self.host.clone(),
            port: self.port,
            auth: self.auth.clone(),
            database: self.database.clone(),
            tls: None,
            timeout,
            read_only: false,
        }
    }
}

/// Parse a connection string into components
///
/// Supported formats:
/// - redis://host:port
/// - redis://host:port/db
/// - redis://:password@host:port
/// - redis://user:password@host:port
/// - redis://user:password@host:port/db
/// - memcached://host:port
/// - etcd://host:port
pub fn parse_connection_string(s: &str) -> Result<ParsedConnection, String> {
    // Determine backend type from scheme
    let (backend_type, rest) = if let Some(rest) = s.strip_prefix("redis://") {
        (BackendType::Redis, rest)
    } else if let Some(rest) = s.strip_prefix("memcached://") {
        (BackendType::Memcached, rest)
    } else if let Some(rest) = s.strip_prefix("etcd://") {
        (BackendType::Etcd, rest)
    } else {
        return Err(format!(
            "Unknown scheme. Expected redis://, memcached://, or etcd://. Got: {}",
            s
        ));
    };

    // Split auth from host:port/db
    let (auth, host_port_db) = if let Some(at_pos) = rest.rfind('@') {
        let auth_part = &rest[..at_pos];
        let host_part = &rest[at_pos + 1..];

        // Parse auth (user:password or :password)
        let auth = if let Some(colon_pos) = auth_part.find(':') {
            let user = &auth_part[..colon_pos];
            let pass = &auth_part[colon_pos + 1..];

            if user.is_empty() {
                // Redis-style :password (ACL default user)
                Some(Auth::UserPassword {
                    username: "default".to_string(),
                    password: pass.to_string(),
                })
            } else {
                Some(Auth::UserPassword {
                    username: user.to_string(),
                    password: pass.to_string(),
                })
            }
        } else {
            // Just a token/password without colon
            Some(Auth::UserPassword {
                username: "default".to_string(),
                password: auth_part.to_string(),
            })
        };

        (auth, host_part)
    } else {
        (None, rest)
    };

    // Split host:port from /db
    let (host_port, database) = if let Some(slash_pos) = host_port_db.find('/') {
        let db_str = &host_port_db[slash_pos + 1..];
        let db = if db_str.is_empty() {
            None
        } else {
            Some(db_str.to_string())
        };
        (&host_port_db[..slash_pos], db)
    } else {
        (host_port_db, None)
    };

    // Parse host:port
    let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
        let host = &host_port[..colon_pos];
        let port_str = &host_port[colon_pos + 1..];
        let port: u16 = port_str
            .parse()
            .map_err(|_| format!("Invalid port number: {}", port_str))?;
        (host.to_string(), port)
    } else {
        // Use default ports
        let default_port = match backend_type {
            BackendType::Redis => 6379,
            BackendType::Memcached => 11211,
            BackendType::Etcd => 2379,
        };
        (host_port.to_string(), default_port)
    };

    if host.is_empty() {
        return Err("Host cannot be empty".to_string());
    }

    Ok(ParsedConnection {
        backend_type,
        host,
        port,
        auth,
        database,
    })
}

/// Generate a simple UUID-like string for temporary connection IDs
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_secs(), now.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_redis_simple() {
        let conn = parse_connection_string("redis://localhost:6379").unwrap();
        assert_eq!(conn.backend_type, BackendType::Redis);
        assert_eq!(conn.host, "localhost");
        assert_eq!(conn.port, 6379);
        assert!(conn.auth.is_none());
        assert!(conn.database.is_none());
    }

    #[test]
    fn test_parse_redis_with_db() {
        let conn = parse_connection_string("redis://localhost:6379/5").unwrap();
        assert_eq!(conn.database, Some("5".to_string()));
    }

    #[test]
    fn test_parse_redis_with_password() {
        let conn = parse_connection_string("redis://:secret@localhost:6379").unwrap();
        match conn.auth {
            Some(Auth::UserPassword { username, password }) => {
                assert_eq!(username, "default");
                assert_eq!(password, "secret");
            }
            _ => panic!("Expected UserPassword auth"),
        }
    }

    #[test]
    fn test_parse_redis_with_user_password() {
        let conn = parse_connection_string("redis://myuser:mypass@localhost:6379/0").unwrap();
        match conn.auth {
            Some(Auth::UserPassword { username, password }) => {
                assert_eq!(username, "myuser");
                assert_eq!(password, "mypass");
            }
            _ => panic!("Expected UserPassword auth"),
        }
        assert_eq!(conn.database, Some("0".to_string()));
    }

    #[test]
    fn test_parse_memcached() {
        let conn = parse_connection_string("memcached://cache.local:11211").unwrap();
        assert_eq!(conn.backend_type, BackendType::Memcached);
        assert_eq!(conn.host, "cache.local");
        assert_eq!(conn.port, 11211);
    }

    #[test]
    fn test_parse_etcd() {
        let conn = parse_connection_string("etcd://etcd.cluster:2379").unwrap();
        assert_eq!(conn.backend_type, BackendType::Etcd);
        assert_eq!(conn.host, "etcd.cluster");
        assert_eq!(conn.port, 2379);
    }

    #[test]
    fn test_parse_default_port() {
        let conn = parse_connection_string("redis://localhost").unwrap();
        assert_eq!(conn.port, 6379);

        let conn = parse_connection_string("memcached://localhost").unwrap();
        assert_eq!(conn.port, 11211);

        let conn = parse_connection_string("etcd://localhost").unwrap();
        assert_eq!(conn.port, 2379);
    }

    #[test]
    fn test_parse_invalid_scheme() {
        let result = parse_connection_string("mysql://localhost:3306");
        assert!(result.is_err());
    }
}
