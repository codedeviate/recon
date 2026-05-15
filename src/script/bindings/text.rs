//! `text::*` static module — charset conversion and text helpers.
//!
//! Thin wrappers over `crate::text_encoding`. Scripts use these to
//! decode response bytes from arbitrary encodings, re-encode strings
//! for legacy APIs, or strip BOMs before parsing.
//!
//! ```text
//! let r = http("https://legacy.example.com/");
//! let utf8 = text::decode(r.body_bytes, r.charset ?? "windows-1252");
//! print(utf8);
//! ```

use crate::script::convert::err;
use crate::text_encoding;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut m = Module::new();

    // ── Conversion ────────────────────────────────────────────────────────
    m.set_native_fn(
        "transcode",
        |bytes: Blob, from: &str, to: &str| -> Result<Blob, Box<EvalAltResult>> {
            let from_enc = text_encoding::resolve(from).map_err(|e| err(format!("text::transcode: {e}")))?;
            let to_enc = text_encoding::resolve(to).map_err(|e| err(format!("text::transcode: {e}")))?;
            let r = text_encoding::transcode(&bytes, from_enc, to_enc);
            Ok(r.bytes)
        },
    );

    m.set_native_fn(
        "decode",
        |bytes: Blob, charset: &str| -> Result<String, Box<EvalAltResult>> {
            let enc = text_encoding::resolve(charset).map_err(|e| err(format!("text::decode: {e}")))?;
            Ok(text_encoding::decode_to_string(&bytes, enc))
        },
    );

    m.set_native_fn(
        "encode",
        |s: &str, charset: &str| -> Result<Blob, Box<EvalAltResult>> {
            let enc = text_encoding::resolve(charset).map_err(|e| err(format!("text::encode: {e}")))?;
            Ok(text_encoding::encode_from_str(s, enc).bytes)
        },
    );

    // ── Detection + helpers ───────────────────────────────────────────────
    m.set_native_fn(
        "detect",
        |bytes: Blob| -> Result<Map, Box<EvalAltResult>> {
            let d = text_encoding::detect(&bytes);
            let mut out = Map::new();
            out.insert("charset".into(), d.charset.to_string().into());
            out.insert("had_bom".into(), d.had_bom.into());
            Ok(out)
        },
    );

    m.set_native_fn(
        "charset_of",
        |headers: Map| -> Result<Dynamic, Box<EvalAltResult>> {
            // Accept either:
            //   - the http() response map shape: headers["content-type"] -> Array<String>
            //   - a simpler map: "content-type" -> String
            // Case-insensitive key match.
            let mut ct: Option<String> = None;
            for (k, v) in headers.iter() {
                if k.eq_ignore_ascii_case("content-type") {
                    if let Some(s) = v.clone().try_cast::<String>() {
                        ct = Some(s);
                    } else if let Some(arr) = v.clone().try_cast::<Array>() {
                        if let Some(first) = arr.first() {
                            ct = first.clone().try_cast::<String>();
                        }
                    }
                    break;
                }
            }
            Ok(match ct.and_then(|v| text_encoding::parse_content_type_charset(&v)) {
                Some(c) => c.into(),
                None => Dynamic::UNIT,
            })
        },
    );

    m.set_native_fn(
        "strip_bom",
        |bytes: Blob| -> Result<Blob, Box<EvalAltResult>> {
            Ok(text_encoding::strip_bom(&bytes).to_vec())
        },
    );

    m.set_native_fn(
        "list",
        || -> Result<Array, Box<EvalAltResult>> {
            Ok(text_encoding::common_labels()
                .iter()
                .map(|s| Dynamic::from(s.to_string()))
                .collect())
        },
    );

    // ── Normalisation ─────────────────────────────────────────────────────
    m.set_native_fn(
        "normalize_newlines",
        |s: &str, style: &str| -> Result<String, Box<EvalAltResult>> {
            let sep = match style.to_ascii_lowercase().as_str() {
                "lf" | "unix" => "\n",
                "crlf" | "windows" | "dos" => "\r\n",
                "cr" | "mac" => "\r",
                other => return Err(err(format!(
                    "text::normalize_newlines: style must be 'lf'/'crlf'/'cr' (got '{other}')"
                ))),
            };
            // Split on any of the three line endings, then rejoin.
            let lines: Vec<&str> = s.split_inclusive(['\n', '\r']).collect();
            let mut out = String::with_capacity(s.len());
            let mut i = 0;
            while i < lines.len() {
                let line = lines[i];
                // Detect existing ending and skip over the paired char on CRLF.
                let (content, saw_newline) = if let Some(stripped) = line.strip_suffix("\r\n") {
                    (stripped, true)
                } else if let Some(stripped) = line.strip_suffix('\n') {
                    (stripped, true)
                } else if let Some(stripped) = line.strip_suffix('\r') {
                    // Either a Mac line ending, or the first half of a CRLF split
                    // across chunks. `split_inclusive` doesn't combine them.
                    if i + 1 < lines.len() && lines[i + 1].starts_with('\n') {
                        out.push_str(stripped);
                        out.push_str(sep);
                        // Skip the orphan \n chunk — but preserve any content after it.
                        let next = &lines[i + 1][1..];
                        out.push_str(next);
                        i += 2;
                        continue;
                    }
                    (stripped, true)
                } else {
                    (line, false)
                };
                out.push_str(content);
                if saw_newline {
                    out.push_str(sep);
                }
                i += 1;
            }
            Ok(out)
        },
    );

    engine.register_static_module("text", m.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn transcode_utf8_to_latin1_and_back() {
        let e = engine();
        let script = r#"
let utf8 = "café".to_blob();
let l1 = text::transcode(utf8, "utf-8", "iso-8859-1");
let back = text::transcode(l1, "iso-8859-1", "utf-8");
let s = text::decode(back, "utf-8");
if s == "café" { 1 } else { 0 }
"#;
        let n: i64 = e.eval(script).expect("eval");
        assert_eq!(n, 1);
    }

    #[test]
    fn decode_latin1_bytes() {
        let e = engine();
        // Build Latin-1 bytes for "café" (é = 0xE9) in Rhai.
        let script = r#"
let b = blob();
b.push(0x63); b.push(0x61); b.push(0x66); b.push(0xE9);
let s = text::decode(b, "iso-8859-1");
if s == "café" { 1 } else { 0 }
"#;
        let n: i64 = e.eval(script).expect("eval");
        assert_eq!(n, 1);
    }

    #[test]
    fn encode_utf8_roundtrip() {
        let e = engine();
        let script = r#"
let bytes = text::encode("hello", "utf-8");
let s = text::decode(bytes, "utf-8");
s == "hello"
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn detect_bom_is_utf8() {
        let e = engine();
        let script = r#"
let b = blob();
b.push(0xEF); b.push(0xBB); b.push(0xBF);
b.push(0x68); b.push(0x69);
let d = text::detect(b);
d.charset == "UTF-8" && d.had_bom == true
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn strip_bom_drops_utf8_bom() {
        let e = engine();
        let script = r#"
let b = blob();
b.push(0xEF); b.push(0xBB); b.push(0xBF);
b.push(0x68); b.push(0x69);
let stripped = text::strip_bom(b);
stripped.len() == 2
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn charset_of_extracts_from_headers_array() {
        let e = engine();
        let script = r#"
let headers = #{ "content-type": ["text/html; charset=iso-8859-1"] };
text::charset_of(headers)
"#;
        let c: String = e.eval(script).expect("eval");
        assert_eq!(c, "iso-8859-1");
    }

    #[test]
    fn charset_of_returns_unit_when_absent() {
        let e = engine();
        let script = r#"
let headers = #{ "content-type": ["application/json"] };
let c = text::charset_of(headers);
c == ()
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn list_is_non_empty() {
        let e = engine();
        let n: i64 = e.eval("text::list().len()").expect("eval");
        assert!(n > 10);
    }

    #[test]
    fn unknown_charset_throws() {
        let e = engine();
        let res: Result<String, _> =
            e.eval(r#"text::decode("x".to_blob(), "definitely-not-a-charset")"#);
        assert!(res.is_err());
    }

    #[test]
    fn normalize_newlines_to_lf() {
        let e = engine();
        let script = r#"text::normalize_newlines("a\r\nb\rc\nd", "lf")"#;
        let s: String = e.eval(script).expect("eval");
        assert_eq!(s, "a\nb\nc\nd");
    }

    #[test]
    fn normalize_newlines_to_crlf() {
        let e = engine();
        let script = r#"text::normalize_newlines("a\nb", "crlf")"#;
        let s: String = e.eval(script).expect("eval");
        assert_eq!(s, "a\r\nb");
    }
}
