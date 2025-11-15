use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub backend_type: BackendType,
    pub host: String,
    pub port: u16,
    pub auth: Option<Auth>,
    pub database: Option<String>,
    pub tls: Option<TlsConfig>,
    pub timeout: Duration,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Redis,
    Memcached,
    Etcd,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Redis => write!(f, "Redis"),
            BackendType::Memcached => write!(f, "Memcached"),
            BackendType::Etcd => write!(f, "etcd"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    /// Username + password
    UserPassword { username: String, password: String },
    /// Token-based (etcd, Consul)
    Token(String),
    /// Certificate-based
    Certificate {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConfig {
    pub enabled: bool,
    pub ca_cert_path: Option<PathBuf>,
    pub verify_hostname: bool,
}
