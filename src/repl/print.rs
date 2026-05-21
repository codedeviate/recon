//! Pretty-printer for REPL auto-print. Formats `rhai::Dynamic` values
//! into a single-line representation suitable for display after a
//! successful eval.
//!
//! Truncation: arrays/maps are formatted recursively but truncated at
//! ~200 visible chars with a trailing `…(N more)` indicator so a giant
//! HTTP response body doesn't fill the terminal.

use rhai::Dynamic;

const MAX_INLINE_LEN: usize = 200;

/// Format a Dynamic for REPL display. Returns None if the value should
/// be suppressed (unit).
pub fn format(value: &Dynamic) -> Option<String> {
    if value.is_unit() {
        return None;
    }
    Some(fmt_inner(value))
}

fn fmt_inner(value: &Dynamic) -> String {
    if let Ok(s) = value.as_immutable_string_ref() {
        return fmt_string(s.as_str());
    }
    if let Ok(b) = value.as_bool() {
        return format!("{b}");
    }
    if let Ok(i) = value.as_int() {
        return format!("{i}");
    }
    if let Ok(f) = value.as_float() {
        return format!("{f:?}");
    }
    if let Ok(c) = value.as_char() {
        return format!("'{}'", c.escape_default());
    }
    if let Some(arr) = value.read_lock::<rhai::Array>() {
        return fmt_array(&arr);
    }
    if let Some(map) = value.read_lock::<rhai::Map>() {
        return fmt_map(&map);
    }
    if let Some(blob) = value.read_lock::<rhai::Blob>() {
        return fmt_blob(&blob);
    }
    format!("<{}>", value.type_name())
}

fn fmt_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    truncate(out)
}

fn fmt_array(arr: &[Dynamic]) -> String {
    let parts: Vec<String> = arr.iter().map(fmt_inner).collect();
    truncate(format!("[{}]", parts.join(", ")))
}

fn fmt_map(map: &rhai::Map) -> String {
    let mut parts: Vec<String> =
        map.iter().map(|(k, v)| format!("{}: {}", k, fmt_inner(v))).collect();
    parts.sort();
    truncate(format!("#{{{}}}", parts.join(", ")))
}

fn fmt_blob(blob: &[u8]) -> String {
    let preview_len = blob.len().min(16);
    let hex: Vec<String> =
        blob[..preview_len].iter().map(|b| format!("{b:02x}")).collect();
    let tail = if blob.len() > preview_len { " …" } else { "" };
    format!("<blob {} bytes: {}{}>", blob.len(), hex.join(" "), tail)
}

fn truncate(s: String) -> String {
    if s.chars().count() <= MAX_INLINE_LEN {
        return s;
    }
    let head: String = s.chars().take(MAX_INLINE_LEN).collect();
    let extra = s.chars().count() - MAX_INLINE_LEN;
    format!("{head}…({extra} more)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Dynamic;

    #[test]
    fn unit_is_none() {
        assert!(format(&Dynamic::UNIT).is_none());
    }

    #[test]
    fn int_formats_bare() {
        assert_eq!(format(&Dynamic::from(42_i64)).unwrap(), "42");
    }

    #[test]
    fn bool_formats_bare() {
        assert_eq!(format(&Dynamic::from(true)).unwrap(), "true");
    }

    #[test]
    fn string_is_quoted_and_escaped() {
        let s = Dynamic::from("hello\nworld".to_string());
        assert_eq!(format(&s).unwrap(), r#""hello\nworld""#);
    }

    #[test]
    fn array_formats_recursive() {
        let arr: rhai::Array = vec![
            Dynamic::from(1_i64),
            Dynamic::from("x".to_string()),
            Dynamic::from(true),
        ];
        assert_eq!(format(&Dynamic::from(arr)).unwrap(), r#"[1, "x", true]"#);
    }

    #[test]
    fn map_formats_sorted() {
        let mut m = rhai::Map::new();
        m.insert("b".into(), Dynamic::from(2_i64));
        m.insert("a".into(), Dynamic::from(1_i64));
        assert_eq!(format(&Dynamic::from(m)).unwrap(), "#{a: 1, b: 2}");
    }

    #[test]
    fn blob_shows_first_16_bytes() {
        let blob: rhai::Blob = b"Hello, world!".to_vec();
        let out = format(&Dynamic::from(blob)).unwrap();
        assert!(out.starts_with("<blob 13 bytes: 48 65 6c 6c"), "{out}");
        assert!(out.ends_with(">"));
    }

    #[test]
    fn long_string_truncates() {
        let big = "x".repeat(500);
        let out = format(&Dynamic::from(big)).unwrap();
        assert!(out.contains("…("), "expected truncation marker: {out}");
    }
}
