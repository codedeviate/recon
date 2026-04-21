//! Hash script bindings — `md5`, `sha1`, `sha256`, `sha384`, `sha512`,
//! `sha3_256`, `sha3_512`, `blake3`, `crc32`, and the generic `hash(algo,
//! x [, format])`.
//!
//! All per-algo functions return lowercase-hex digests by default. Input
//! is either a UTF-8 String or a Rhai Blob (`Vec<u8>`); other types raise
//! a Rhai exception. Use the generic `hash(algo, x, format)` form for
//! base64 output.
//!
//! The bindings delegate to `hash::digest_string` so the CLI and scripts
//! share a single implementation path.

use crate::hash::{self, Algo, Format};
use crate::script::convert::{anyhow_to_rhai, err};
use rhai::{Blob, Engine, EvalAltResult};

pub fn register(engine: &mut Engine) {
    // One entry per (algo, input-type) pair. Each algo accepts String AND
    // Blob input so scripts can do `md5(file_read(path))` without a cast.
    for (name, algo) in algo_pairs().iter().copied() {
        engine.register_fn(name, move |s: &str| -> Result<String, Box<EvalAltResult>> {
            hash::digest_string(algo, s.as_bytes(), Format::Hex).map_err(anyhow_to_rhai)
        });
        engine.register_fn(
            name,
            move |b: Blob| -> Result<String, Box<EvalAltResult>> {
                hash::digest_string(algo, &b, Format::Hex).map_err(anyhow_to_rhai)
            },
        );
    }

    // Generic: hash(algo, x) — hex output.
    engine.register_fn(
        "hash",
        |algo: &str, s: &str| -> Result<String, Box<EvalAltResult>> {
            let a = hash::parse_algo(algo).map_err(anyhow_to_rhai)?;
            hash::digest_string(a, s.as_bytes(), Format::Hex).map_err(anyhow_to_rhai)
        },
    );
    engine.register_fn(
        "hash",
        |algo: &str, b: Blob| -> Result<String, Box<EvalAltResult>> {
            let a = hash::parse_algo(algo).map_err(anyhow_to_rhai)?;
            hash::digest_string(a, &b, Format::Hex).map_err(anyhow_to_rhai)
        },
    );

    // Generic with format: hash(algo, x, format). Format is "hex" or
    // "base64" (script-visible; "raw" is disallowed — it produces garbled
    // strings that scripts would almost certainly misuse).
    engine.register_fn(
        "hash",
        |algo: &str, s: &str, format: &str| -> Result<String, Box<EvalAltResult>> {
            let a = hash::parse_algo(algo).map_err(anyhow_to_rhai)?;
            let f = script_format(format)?;
            hash::digest_string(a, s.as_bytes(), f).map_err(anyhow_to_rhai)
        },
    );
    engine.register_fn(
        "hash",
        |algo: &str, b: Blob, format: &str| -> Result<String, Box<EvalAltResult>> {
            let a = hash::parse_algo(algo).map_err(anyhow_to_rhai)?;
            let f = script_format(format)?;
            hash::digest_string(a, &b, f).map_err(anyhow_to_rhai)
        },
    );
}

/// Map every supported algo to its script-visible function name. Per-algo
/// names use underscores where the canonical name uses hyphens (Rhai
/// identifiers can't have hyphens).
fn algo_pairs() -> &'static [(&'static str, Algo)] {
    &[
        ("md5", Algo::Md5),
        ("sha1", Algo::Sha1),
        ("sha256", Algo::Sha256),
        ("sha384", Algo::Sha384),
        ("sha512", Algo::Sha512),
        ("sha3_256", Algo::Sha3_256),
        ("sha3_512", Algo::Sha3_512),
        ("blake3", Algo::Blake3),
        ("crc32", Algo::Crc32),
    ]
}

fn script_format(name: &str) -> Result<Format, Box<EvalAltResult>> {
    match name.trim().to_ascii_lowercase().as_str() {
        "hex" => Ok(Format::Hex),
        "base64" | "b64" => Ok(Format::Base64),
        other => Err(err(format!(
            "hash: unknown format '{other}', expected \"hex\" or \"base64\""
        ))),
    }
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
    fn md5_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"md5("hello")"#).expect("eval");
        assert_eq!(h, "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn sha256_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"sha256("hello")"#).expect("eval");
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha1_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"sha1("hello")"#).expect("eval");
        assert_eq!(h, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
    }

    #[test]
    fn sha384_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"sha384("hello")"#).expect("eval");
        assert_eq!(h.len(), 96);
    }

    #[test]
    fn sha512_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"sha512("hello")"#).expect("eval");
        assert_eq!(h.len(), 128);
    }

    #[test]
    fn sha3_256_and_sha3_512_registered() {
        let e = engine();
        let a: String = e.eval(r#"sha3_256("hello")"#).expect("sha3_256 eval");
        let b: String = e.eval(r#"sha3_512("hello")"#).expect("sha3_512 eval");
        assert_eq!(a.len(), 64);
        assert_eq!(b.len(), 128);
    }

    #[test]
    fn blake3_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"blake3("hello")"#).expect("eval");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn crc32_hex_of_hello() {
        let e = engine();
        let h: String = e.eval(r#"crc32("hello")"#).expect("eval");
        assert_eq!(h, "3610a686");
    }

    #[test]
    fn generic_hash_matches_per_algo() {
        let e = engine();
        let a: String = e.eval(r#"sha256("hello")"#).unwrap();
        let b: String = e.eval(r#"hash("sha256", "hello")"#).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn generic_hash_base64() {
        let e = engine();
        let h: String = e
            .eval(r#"hash("sha1", "hello", "base64")"#)
            .expect("eval");
        assert_eq!(h, "qvTGHdzF6KLavt4PO0gs2a6pQ00=");
    }

    #[test]
    fn hash_of_blob_input() {
        let e = engine();
        // Build a blob from string bytes via file_read-style path: use the
        // blob() constructor Rhai exposes, then push bytes.
        let script = r#"
let b = blob();
b.push(104); b.push(101); b.push(108); b.push(108); b.push(111); // "hello"
md5(b)
"#;
        let h: String = e.eval(script).expect("eval");
        assert_eq!(h, "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn unknown_algo_throws() {
        let e = engine();
        let res: Result<String, _> = e.eval(r#"hash("md100", "x")"#);
        assert!(res.is_err());
    }

    #[test]
    fn unknown_format_throws() {
        let e = engine();
        let res: Result<String, _> = e.eval(r#"hash("md5", "x", "oct")"#);
        assert!(res.is_err());
    }
}
