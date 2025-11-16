use crate::backend::utils::is_json;
use crate::types::Value;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use serde_json;

use super::{FormatError, Formatter};

/// Configuration for JSON formatting and coloring
#[derive(Debug, Clone)]
pub struct JsonColorConfig {
    pub indent: usize,
    pub key_color: Color,
    pub string_color: Color,
    pub number_color: Color,
    pub boolean_color: Color,
    pub null_color: Color,
    pub brace_color: Color,
    pub bracket_color: Color,
    pub comma_color: Color,
    pub colon_color: Color,
}

impl Default for JsonColorConfig {
    fn default() -> Self {
        Self {
            indent: 2,
            key_color: Color::Cyan,
            string_color: Color::Green,
            number_color: Color::Magenta,
            boolean_color: Color::Yellow,
            null_color: Color::DarkGray,
            brace_color: Color::White,
            bracket_color: Color::White,
            comma_color: Color::White,
            colon_color: Color::White,
        }
    }
}

/// JSON formatter with syntax highlighting for ratatui
pub struct JsonFormatter {
    config: JsonColorConfig,
}

impl JsonFormatter {
    pub fn new(config: JsonColorConfig) -> Self {
        Self { config }
    }

    /// Format JSON into styled lines for ratatui
    pub fn format_to_lines(&self, value: &Value) -> Result<Vec<Line<'static>>, FormatError> {
        let json_str = String::from_utf8(value.data.clone()).map_err(|_| FormatError {
            message: "Invalid UTF-8".to_string(),
        })?;

        // Parse JSON to validate and prettify
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| FormatError {
                message: format!("Invalid JSON: {}", e),
            })?;

        // Format with indentation
        let pretty = serde_json::to_string_pretty(&parsed).map_err(|e| FormatError {
            message: format!("Failed to format JSON: {}", e),
        })?;

        // Colorize the formatted JSON
        Ok(self.colorize_json(&pretty))
    }

    /// Colorize JSON string into styled lines
    fn colorize_json(&self, json: &str) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for line in json.lines() {
            let spans = self.colorize_line(line);
            lines.push(Line::from(spans));
        }

        lines
    }

    /// Colorize a single line of JSON
    fn colorize_line(&self, line: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut in_key = false;
        let mut escape_next = false;
        let mut chars = line.chars().peekable();

        // Track if we're in a key position (after : we're in value)
        let mut after_colon = false;

        #[allow(clippy::while_let_on_iterator)]
        while let Some(ch) = chars.next() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    current.push(ch);
                    escape_next = true;
                }
                '"' => {
                    if in_string {
                        // End of string
                        current.push(ch);
                        let color = if in_key {
                            self.config.key_color
                        } else {
                            self.config.string_color
                        };
                        spans.push(Span::styled(current.clone(), Style::default().fg(color)));
                        current.clear();
                        in_string = false;
                        in_key = false;
                    } else {
                        // Flush any accumulated text
                        if !current.is_empty() {
                            spans.push(Span::raw(current.clone()));
                            current.clear();
                        }
                        // Start of string - check if it's a key or value
                        in_string = true;
                        in_key = !after_colon;
                        current.push(ch);
                    }
                }
                ':' if !in_string => {
                    // Flush any accumulated text
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(self.config.colon_color),
                    ));
                    after_colon = true;
                }
                ',' if !in_string => {
                    // Flush any accumulated text (might be a number or boolean)
                    self.flush_value(&mut current, &mut spans, after_colon);
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(self.config.comma_color),
                    ));
                    after_colon = false;
                }
                '{' | '}' if !in_string => {
                    // Flush any accumulated text
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(self.config.brace_color),
                    ));
                    after_colon = false;
                }
                '[' | ']' if !in_string => {
                    // Flush any accumulated text
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(self.config.bracket_color),
                    ));
                    after_colon = false;
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Flush any remaining text
        self.flush_value(&mut current, &mut spans, after_colon);

        spans
    }

    /// Flush accumulated value text with appropriate coloring
    fn flush_value(&self, current: &mut String, spans: &mut Vec<Span<'static>>, after_colon: bool) {
        if current.is_empty() {
            return;
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() {
            // Determine color based on content
            let color = if after_colon {
                if trimmed == "true" || trimmed == "false" {
                    self.config.boolean_color
                } else if trimmed == "null" {
                    self.config.null_color
                } else if trimmed.parse::<f64>().is_ok() {
                    self.config.number_color
                } else {
                    Color::White // Default for whitespace and other chars
                }
            } else {
                Color::White // Default
            };

            if color != Color::White {
                // Keep leading/trailing whitespace separate
                let leading_ws = current.len() - current.trim_start().len();
                let trailing_ws = current.len() - current.trim_end().len();

                if leading_ws > 0 {
                    spans.push(Span::raw(current[..leading_ws].to_string()));
                }
                spans.push(Span::styled(
                    trimmed.to_string(),
                    Style::default().fg(color),
                ));
                if trailing_ws > 0 {
                    spans.push(Span::raw(
                        current[current.len() - trailing_ws..].to_string(),
                    ));
                }
            } else {
                spans.push(Span::raw(current.clone()));
            }
        } else {
            spans.push(Span::raw(current.clone()));
        }

        current.clear();
    }
}

impl Formatter for JsonFormatter {
    fn name(&self) -> &str {
        "json"
    }

    fn can_format(&self, value: &Value) -> bool {
        is_json(&value.data)
    }

    fn format(&self, value: &Value) -> Result<String, FormatError> {
        let json_str = String::from_utf8(value.data.clone()).map_err(|_| FormatError {
            message: "Invalid UTF-8".to_string(),
        })?;

        // Parse and prettify JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| FormatError {
                message: format!("Invalid JSON: {}", e),
            })?;

        serde_json::to_string_pretty(&parsed).map_err(|e| FormatError {
            message: format!("Failed to format JSON: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatter_can_format() {
        use crate::types::ValueType;

        let formatter = JsonFormatter::new(JsonColorConfig::default());
        let json_value = Value {
            data: b"{\"key\": \"value\"}".to_vec(),
            value_type: ValueType::Json,
            encoding: None,
        };
        assert!(formatter.can_format(&json_value));

        let text_value = Value {
            data: b"plain text".to_vec(),
            value_type: ValueType::String,
            encoding: None,
        };
        assert!(!formatter.can_format(&text_value));
    }

    #[test]
    fn test_json_formatter_format() {
        use crate::types::ValueType;

        let formatter = JsonFormatter::new(JsonColorConfig::default());
        let json_value = Value {
            data: b"{\"key\":\"value\",\"num\":123}".to_vec(),
            value_type: ValueType::Json,
            encoding: None,
        };
        let result = formatter.format(&json_value);
        assert!(result.is_ok());
        let formatted = result.unwrap();
        assert!(formatted.contains("\"key\""));
        assert!(formatted.contains("\"value\""));
        assert!(formatted.contains("123"));
    }

    #[test]
    fn test_json_formatter_format_invalid_json() {
        use crate::types::ValueType;

        let formatter = JsonFormatter::new(JsonColorConfig::default());
        let invalid_value = Value {
            data: b"{invalid}".to_vec(),
            value_type: ValueType::Json,
            encoding: None,
        };
        let result = formatter.format(&invalid_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_formatter_format_to_lines() {
        use crate::types::ValueType;

        let formatter = JsonFormatter::new(JsonColorConfig::default());
        let json_value = Value {
            data: br#"{"key":"value","num":123,"bool":true,"null":null}"#.to_vec(),
            value_type: ValueType::Json,
            encoding: None,
        };
        let result = formatter.format_to_lines(&json_value);
        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
    }
}
