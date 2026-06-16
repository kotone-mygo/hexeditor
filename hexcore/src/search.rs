pub struct Searcher;

impl Searcher {
    pub fn find_text(data: &[u8], pattern: &str, case_sensitive: bool) -> Vec<u64> {
        if pattern.is_empty() {
            return Vec::new();
        }
        let pat_bytes = pattern.as_bytes();
        let pat_len = pat_bytes.len();
        if data.len() < pat_len {
            return Vec::new();
        }
        let mut results = Vec::new();
        if case_sensitive {
            for (i, window) in data.windows(pat_len).enumerate() {
                if window == pat_bytes {
                    results.push(i as u64);
                }
            }
        } else {
            let pat_lower: Vec<u8> = pat_bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
            for (i, window) in data.windows(pat_len).enumerate() {
                let mut matched = true;
                for (j, &byte) in window.iter().enumerate() {
                    if byte.to_ascii_lowercase() != pat_lower[j] {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    results.push(i as u64);
                }
            }
        }
        results
    }

    pub fn find_hex(data: &[u8], pattern: &str) -> Vec<u64> {
        let parsed = Self::parse_hex_pattern(pattern);
        let pat = match parsed {
            Some(p) => p,
            None => return Vec::new(),
        };
        let pat_len = pat.len();
        if data.len() < pat_len {
            return Vec::new();
        }
        let mut results = Vec::new();
        for (i, window) in data.windows(pat_len).enumerate() {
            let mut matched = true;
            for (j, &byte) in window.iter().enumerate() {
                match pat[j] {
                    Some(expected) => {
                        if byte != expected {
                            matched = false;
                            break;
                        }
                    }
                    None => {}
                }
            }
            if matched {
                results.push(i as u64);
            }
        }
        results
    }

    pub fn parse_hex_pattern(pattern: &str) -> Option<Vec<Option<u8>>> {
        let mut result = Vec::new();
        for token in pattern.split_whitespace() {
            if token == "??" {
                result.push(None);
            } else {
                let hex = token.strip_prefix("0x").unwrap_or(token);
                if hex.len() == 2 {
                    let byte = u8::from_str_radix(hex, 16).ok()?;
                    result.push(Some(byte));
                } else if hex.len() > 2 && hex.len() % 2 == 0 {
                    for i in (0..hex.len()).step_by(2) {
                        let byte = u8::from_str_radix(&hex[i..i + 2], 16).ok()?;
                        result.push(Some(byte));
                    }
                } else {
                    return None;
                }
            }
        }
        if result.is_empty() {
            return None;
        }
        Some(result)
    }

    pub fn hex_to_bytes(pattern: &str) -> Option<Vec<u8>> {
        let parsed = Self::parse_hex_pattern(pattern)?;
        let mut bytes = Vec::with_capacity(parsed.len());
        for b in parsed {
            bytes.push(b?);
        }
        Some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_text_basic() {
        let data = b"Hello World Hello";
        let results = Searcher::find_text(data, "Hello", true);
        assert_eq!(results, vec![0, 12]);
    }

    #[test]
    fn test_find_text_not_found() {
        let data = b"Hello World";
        let results = Searcher::find_text(data, "xyz", true);
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_text_case_insensitive() {
        let data = b"hello HELLO Hello";
        let results = Searcher::find_text(data, "hello", false);
        assert_eq!(results, vec![0, 6, 12]);
    }

    #[test]
    fn test_find_text_empty_pattern() {
        let data = b"hello";
        let results = Searcher::find_text(data, "", true);
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_hex_with_wildcard() {
        let data = &[0x48, 0x65, 0x6C, 0x6C, 0x6F];
        let results = Searcher::find_hex(data, "48 65 ?? 6C");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_find_hex_literal() {
        let data = &[0x00, 0xFF, 0xFE, 0x00, 0xFF];
        let results = Searcher::find_hex(data, "FF FE");
        assert_eq!(results, vec![1]);
    }

    #[test]
    fn test_find_hex_not_found() {
        let data = &[0x00, 0x01, 0x02];
        let results = Searcher::find_hex(data, "FF FF");
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_hex_0x_prefix() {
        let data = &[0x48, 0x65, 0x6C];
        let results = Searcher::find_hex(data, "0x48 0x65");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_find_hex_invalid_pattern() {
        let data = &[0x00, 0x01];
        let results = Searcher::find_hex(data, "ZZ");
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_hex_contiguous() {
        let data = &[0x48, 0x65, 0x6C, 0x6C, 0x6F];
        let results = Searcher::find_hex(data, "48656C");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_find_hex_contiguous_with_0x_prefix() {
        let data = &[0x48, 0x65, 0x6C];
        let results = Searcher::find_hex(data, "0x48656C");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_find_hex_contiguous_mixed_whitespace() {
        let data = &[0x48, 0x65, 0x6C, 0x6C, 0x6F];
        let results = Searcher::find_hex(data, "48 656C");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_find_hex_contiguous_odd_length_falls_through() {
        let data = &[0x48, 0x65, 0x6C];
        let results = Searcher::find_hex(data, "4865C");
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_hex_text_abcd_still_falls_through_for_nonhex() {
        let data = b"xyz not hex";
        let results = Searcher::find_hex(data, "xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_hex_to_bytes_basic() {
        let bytes = Searcher::hex_to_bytes("48656C").unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6C]);
    }

    #[test]
    fn test_hex_to_bytes_with_whitespace() {
        let bytes = Searcher::hex_to_bytes("48 65 6C").unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6C]);
    }

    #[test]
    fn test_hex_to_bytes_rejects_wildcard() {
        let result = Searcher::hex_to_bytes("48 ?? 6C");
        assert!(result.is_none());
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        let result = Searcher::hex_to_bytes("ZZ");
        assert!(result.is_none());
    }
}
