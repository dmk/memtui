/// Detect if data is JSON by checking if it starts/ends with JSON delimiters
///
pub fn is_json(data: &[u8]) -> bool {
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_json_object() {
        assert!(is_json(b"{\"key\": \"value\"}"));
        assert!(is_json(b"  {\"key\": \"value\"}  "));
    }

    #[test]
    fn test_is_json_array() {
        assert!(is_json(b"[1, 2, 3]"));
        assert!(is_json(b"  [1, 2, 3]  "));
    }

    #[test]
    fn test_is_not_json() {
        assert!(!is_json(b"plain string"));
        assert!(!is_json(b"123"));
        assert!(!is_json(b"{incomplete"));
    }

    #[test]
    fn test_is_json_invalid_utf8() {
        assert!(!is_json(&[0xFF, 0xFE, 0xFD]));
    }
}
