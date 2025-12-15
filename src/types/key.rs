use super::ValueType;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub name: String,
    pub value_type: ValueType,
    pub size_bytes: u64,
    pub ttl: Option<Duration>,
    pub last_accessed: Option<SystemTime>,
    pub encoding: Option<String>,
    pub expires_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct KeyScanResult {
    pub keys: Vec<KeyMetadata>,
    pub cursor: Option<String>,
    pub has_more: bool,
}
