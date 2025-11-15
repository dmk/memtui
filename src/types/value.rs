#[derive(Debug, Clone)]
pub struct Value {
    pub data: Vec<u8>,
    pub value_type: ValueType,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Binary,
    Json,
    Integer,
    Float,
    List,
    Set,
    Hash,
    SortedSet,
    Unknown,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::String => write!(f, "string"),
            ValueType::Binary => write!(f, "binary"),
            ValueType::Json => write!(f, "json"),
            ValueType::Integer => write!(f, "int"),
            ValueType::Float => write!(f, "float"),
            ValueType::List => write!(f, "list"),
            ValueType::Set => write!(f, "set"),
            ValueType::Hash => write!(f, "hash"),
            ValueType::SortedSet => write!(f, "zset"),
            ValueType::Unknown => write!(f, "unknown"),
        }
    }
}
