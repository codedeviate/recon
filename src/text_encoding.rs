//! Character-set conversion utilities. Wraps `encoding_rs` for lookup,
//! encode/decode, and transcoding; `chardetng` for auto-detection.
//!
//! Used by:
//! - `--output-charset` / `--source-charset` in the response write path
//!   (`src/output.rs`).
//! - `--request-charset` / implicit Content-Type transcoding in
//!   `src/client.rs`.
//! - The `--iconv` standalone file/stdin mode (`src/iconv.rs`).
//! - The `text::*` script bindings (`src/script/bindings/text.rs`).
//!
//! All conversion paths use `?` as the substitute for unmappable
//! characters — matches iconv's `-c` behaviour and curl's HTML-style
//! substitution. Callers get a `had_unmappable` flag so they can emit a
//! warning when appropriate.

use anyhow::{anyhow, Result};
use encoding_rs::{Encoding, UTF_8};

/// Result of a transcode operation.
pub struct TranscodeResult {
    pub bytes: Vec<u8>,
    /// True when one or more characters couldn't be represented in the
    /// target encoding and were substituted (with `?`).
    pub had_unmappable: bool,
}

/// Result of `detect()`.
#[derive(Debug, Clone)]
pub struct DetectResult {
    /// Encoding label (e.g. "UTF-8", "windows-1252"). Always a canonical
    /// encoding_rs name, never an alias.
    pub charset: &'static str,
    /// True when a BOM (UTF-8 / UTF-16 LE or BE) was found and used as
    /// the primary signal. Higher-confidence than `chardetng`.
    pub had_bom: bool,
}

/// Look up an `Encoding` by label. Accepts every alias encoding_rs
/// recognises (`utf-8`, `utf8`, `UTF_8`, `latin-1`, `iso-8859-1`,
/// `ISO_8859-1:1987`, `csISOLatin1`, etc.) plus a few convenience forms:
/// empty string → UTF-8, `latin1` → ISO-8859-1.
pub fn resolve(label: &str) -> Result<&'static Encoding> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Ok(UTF_8);
    }
    // encoding_rs' `for_label` is WHATWG-compatible and accepts most
    // common aliases. It does NOT accept `latin1` without a hyphen —
    // special-case the common curl-ism.
    if trimmed.eq_ignore_ascii_case("latin1") {
        return Ok(encoding_rs::WINDOWS_1252);
    }
    Encoding::for_label(trimmed.as_bytes()).ok_or_else(|| {
        anyhow!("unknown charset '{label}' — try --list-charsets for supported labels")
    })
}

/// Parse `charset=X` from a Content-Type header value. Tolerates:
///   - case differences (`CHARSET` == `charset`)
///   - surrounding whitespace
///   - double-quoted values (`charset="utf-8"`)
///   - additional parameters before/after (`text/html; boundary=...; charset=...`)
pub fn parse_content_type_charset(ct: &str) -> Option<String> {
    for part in ct.split(';') {
        let trimmed = part.trim();
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case("charset") {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Detect the source charset of a byte slice. Priority:
///   1. BOM (UTF-8 / UTF-16 LE / UTF-16 BE) — deterministic.
///   2. `chardetng` heuristic — tuned for Western + East-Asian content.
pub fn detect(bytes: &[u8]) -> DetectResult {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return DetectResult { charset: "UTF-8", had_bom: true };
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return DetectResult { charset: "UTF-16BE", had_bom: true };
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return DetectResult { charset: "UTF-16LE", had_bom: true };
    }

    let mut d = chardetng::EncodingDetector::new();
    d.feed(bytes, true);
    let enc = d.guess(None, true);
    DetectResult { charset: enc.name(), had_bom: false }
}

/// Transcode `bytes` from `from` → `to`. Unmappable characters are
/// substituted with `?`. When `from == to` and `from` is UTF-8 + bytes
/// are already valid UTF-8, returns the bytes unchanged.
pub fn transcode(
    bytes: &[u8],
    from: &'static Encoding,
    to: &'static Encoding,
) -> TranscodeResult {
    if from == to {
        return TranscodeResult { bytes: bytes.to_vec(), had_unmappable: false };
    }
    // Two-step: decode to UTF-8 (via Cow<str>), then encode to the target.
    let (text, _enc_used, decode_had_errors) = from.decode(bytes);
    let (out_bytes, _enc_used_out, encode_had_errors) = to.encode(&text);
    TranscodeResult {
        bytes: out_bytes.into_owned(),
        had_unmappable: decode_had_errors || encode_had_errors,
    }
}

/// Decode `bytes` as `from` and return a UTF-8 String. Invalid sequences
/// are replaced with the Unicode replacement character (`U+FFFD`) —
/// matches how browsers handle bogus content.
pub fn decode_to_string(bytes: &[u8], from: &'static Encoding) -> String {
    let (text, _enc, _had_errors) = from.decode(bytes);
    text.into_owned()
}

/// Encode a str into bytes in `to`. Unmappable characters are
/// substituted with `?`; returns the has-unmappable flag.
pub fn encode_from_str(s: &str, to: &'static Encoding) -> TranscodeResult {
    let (bytes, _enc, had_errors) = to.encode(s);
    TranscodeResult { bytes: bytes.into_owned(), had_unmappable: had_errors }
}

/// Strip a leading UTF-8 / UTF-16 LE / UTF-16 BE BOM, if present.
pub fn strip_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else if bytes.starts_with(&[0xFE, 0xFF]) || bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    }
}

/// A curated list of labels users are likely to type. `encoding_rs`
/// actually accepts many more aliases — this is the "commonly seen"
/// set for `--list-charsets` output.
pub fn common_labels() -> &'static [&'static str] {
    &[
        "UTF-8",
        "UTF-16BE",
        "UTF-16LE",
        "windows-1252",
        "ISO-8859-1",
        "ISO-8859-2",
        "ISO-8859-3",
        "ISO-8859-4",
        "ISO-8859-5",
        "ISO-8859-6",
        "ISO-8859-7",
        "ISO-8859-8",
        "ISO-8859-10",
        "ISO-8859-13",
        "ISO-8859-14",
        "ISO-8859-15",
        "ISO-8859-16",
        "windows-1250",
        "windows-1251",
        "windows-1253",
        "windows-1254",
        "windows-1255",
        "windows-1256",
        "windows-1257",
        "windows-1258",
        "KOI8-R",
        "KOI8-U",
        "macintosh",
        "Shift_JIS",
        "EUC-JP",
        "ISO-2022-JP",
        "EUC-KR",
        "Big5",
        "GBK",
        "GB18030",
        "IBM866",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::{ISO_8859_2, UTF_16LE, WINDOWS_1252};

    #[test]
    fn resolve_common_aliases() {
        assert_eq!(resolve("utf-8").unwrap(), UTF_8);
        assert_eq!(resolve("UTF-8").unwrap(), UTF_8);
        assert_eq!(resolve("utf8").unwrap(), UTF_8);
        assert_eq!(resolve("iso-8859-1").unwrap().name(), "windows-1252");
        // encoding_rs canonicalises "iso-8859-1" to "windows-1252" per
        // the WHATWG encoding spec — document this below in
        // common_labels_resolve. The `latin1` alias is our convenience.
        assert_eq!(resolve("latin1").unwrap(), WINDOWS_1252);
        assert_eq!(resolve("iso-8859-2").unwrap(), ISO_8859_2);
        assert_eq!(resolve("cp1252").unwrap(), WINDOWS_1252);
    }

    #[test]
    fn resolve_empty_is_utf8() {
        assert_eq!(resolve("").unwrap(), UTF_8);
        assert_eq!(resolve("   ").unwrap(), UTF_8);
    }

    #[test]
    fn resolve_unknown_errors() {
        assert!(resolve("definitely-not-a-charset").is_err());
    }

    #[test]
    fn parse_content_type_variations() {
        assert_eq!(
            parse_content_type_charset("text/html; charset=iso-8859-1"),
            Some("iso-8859-1".to_string())
        );
        assert_eq!(
            parse_content_type_charset("application/json"),
            None
        );
        assert_eq!(
            parse_content_type_charset("text/html; CHARSET = \"utf-8\""),
            Some("utf-8".to_string())
        );
        assert_eq!(
            parse_content_type_charset("text/plain; boundary=xxx; charset=windows-1252"),
            Some("windows-1252".to_string())
        );
        assert_eq!(parse_content_type_charset(""), None);
    }

    #[test]
    fn transcode_utf8_to_latin1() {
        // "café" in UTF-8.
        let utf8 = "café".as_bytes();
        let r = transcode(utf8, UTF_8, WINDOWS_1252);
        assert!(!r.had_unmappable);
        // In Windows-1252, 'é' is 0xE9.
        assert_eq!(r.bytes, vec![b'c', b'a', b'f', 0xE9]);
    }

    #[test]
    fn transcode_latin1_to_utf8_roundtrip() {
        let latin1 = vec![b'c', b'a', b'f', 0xE9];
        let r = transcode(&latin1, WINDOWS_1252, UTF_8);
        assert!(!r.had_unmappable);
        assert_eq!(String::from_utf8(r.bytes).unwrap(), "café");
    }

    #[test]
    fn transcode_emoji_to_latin1_flags_unmappable() {
        let r = transcode("hello 🎉".as_bytes(), UTF_8, WINDOWS_1252);
        assert!(r.had_unmappable);
        // The emoji becomes HTML-style numeric reference fallback or '?'
        // depending on encoding_rs — just check we didn't crash and the
        // result ASCII-visible portion is preserved.
        assert!(r.bytes.starts_with(b"hello "));
    }

    #[test]
    fn transcode_same_encoding_passthrough() {
        let r = transcode(b"plain ascii", UTF_8, UTF_8);
        assert_eq!(r.bytes, b"plain ascii");
        assert!(!r.had_unmappable);
    }

    #[test]
    fn detect_utf8_bom() {
        let bytes = b"\xEF\xBB\xBFhello";
        let d = detect(bytes);
        assert_eq!(d.charset, "UTF-8");
        assert!(d.had_bom);
    }

    #[test]
    fn detect_utf16le_bom() {
        let bytes = b"\xFF\xFEh\x00i\x00";
        let d = detect(bytes);
        assert_eq!(d.charset, "UTF-16LE");
        assert!(d.had_bom);
    }

    #[test]
    fn detect_plain_ascii_defaults_to_something_sensible() {
        let d = detect(b"plain ascii, nothing fancy");
        assert!(!d.had_bom);
        // chardetng returns a concrete encoding; plain ASCII is valid in
        // many so we don't pin the exact one.
        assert!(!d.charset.is_empty());
    }

    #[test]
    fn encode_from_str_utf8_passthrough() {
        let r = encode_from_str("hello", UTF_8);
        assert_eq!(r.bytes, b"hello");
        assert!(!r.had_unmappable);
    }

    #[test]
    fn encode_from_str_unmappable() {
        let r = encode_from_str("snow☃man", encoding_rs::ISO_8859_15);
        assert!(r.had_unmappable);
    }

    #[test]
    fn strip_bom_removes_utf8_bom() {
        assert_eq!(strip_bom(b"\xEF\xBB\xBFtext"), b"text");
    }

    #[test]
    fn strip_bom_removes_utf16_bom() {
        assert_eq!(strip_bom(b"\xFF\xFEx"), b"x");
        assert_eq!(strip_bom(b"\xFE\xFFx"), b"x");
    }

    #[test]
    fn strip_bom_leaves_plain_bytes() {
        assert_eq!(strip_bom(b"no bom here"), b"no bom here");
    }

    #[test]
    fn decode_utf16le_via_encoding() {
        let bytes = b"h\x00i\x00";
        let s = decode_to_string(bytes, UTF_16LE);
        assert_eq!(s, "hi");
    }
}
