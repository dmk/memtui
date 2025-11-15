use crate::types::Value;

/// Value formatter trait for display
pub trait Formatter: Send + Sync {
    /// Name of the formatter (e.g., "json", "text")
    fn name(&self) -> &str;

    /// Can this formatter handle this value?
    fn can_format(&self, value: &Value) -> bool;

    /// Format value for display (plain text for now, colors later)
    fn format(&self, value: &Value) -> Result<String, FormatError>;
}

#[derive(Debug)]
pub struct FormatError {
    pub message: String,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Format error: {}", self.message)
    }
}

impl std::error::Error for FormatError {}

/// Simple text formatter (default)
pub struct TextFormatter;

impl Formatter for TextFormatter {
    fn name(&self) -> &str {
        "text"
    }

    fn can_format(&self, _value: &Value) -> bool {
        true // Can handle anything
    }

    fn format(&self, value: &Value) -> Result<String, FormatError> {
        String::from_utf8(value.data.clone()).map_err(|_| FormatError {
            message: "Invalid UTF-8".to_string(),
        })
    }
}
