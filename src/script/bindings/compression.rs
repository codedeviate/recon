//! `compression::*` static module — Rhai bindings for the stream-compression
//! machinery recon already exposes via `--compress` / `--decompress`.
//!
//! All nine algorithms are available: gzip, deflate, zstd, brotli, bzip2,
//! lz4, xz, snappy, zlib (plus their aliases). Input and output are Rhai
//! Blobs (`Vec<u8>`) so scripts can round-trip binary data cleanly.

use crate::compression::{self, Algo, Level};
use crate::script::convert::err;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Map, Module};
use std::io::Cursor;

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    // compression::compress(algo, blob) -> Blob  (default level)
    let _ = module.set_native_fn(
        "compress",
        |algo: &str, data: Blob| -> Result<Blob, Box<EvalAltResult>> {
            let algo = compression::parse_algo(algo).map_err(|e| err(e.to_string()))?;
            do_compress(algo, algo.default_level(), data)
        },
    );

    // compression::compress(algo, blob, level_int) -> Blob
    let _ = module.set_native_fn(
        "compress",
        |algo: &str, data: Blob, level: i64| -> Result<Blob, Box<EvalAltResult>> {
            let algo = compression::parse_algo(algo).map_err(|e| err(e.to_string()))?;
            if algo.is_levelless() {
                return Err(err(format!(
                    "{}: algorithm has no level setting",
                    algo.canonical()
                )));
            }
            if level < 0 {
                return Err(err("compression: level must be non-negative"));
            }
            let resolved =
                compression::resolve_native_level(algo, Level::Num(level as u32))
                    .map_err(|e| err(e.to_string()))?;
            do_compress(algo, resolved, data)
        },
    );

    // compression::compress(algo, blob, word) -> Blob  (fastest/fast/default/good/best)
    let _ = module.set_native_fn(
        "compress",
        |algo: &str, data: Blob, word: &str| -> Result<Blob, Box<EvalAltResult>> {
            let algo = compression::parse_algo(algo).map_err(|e| err(e.to_string()))?;
            let level = compression::parse_level(word).map_err(|e| err(e.to_string()))?;
            if algo.is_levelless() {
                return Err(err(format!(
                    "{}: algorithm has no level setting",
                    algo.canonical()
                )));
            }
            let resolved = compression::resolve_native_level(algo, level)
                .map_err(|e| err(e.to_string()))?;
            do_compress(algo, resolved, data)
        },
    );

    // compression::decompress(blob) -> Blob  (auto-detect from magic bytes)
    let _ = module.set_native_fn(
        "decompress",
        |data: Blob| -> Result<Blob, Box<EvalAltResult>> {
            let algo = compression::detect_from_magic(&data).ok_or_else(|| {
                err("compression: could not auto-detect algorithm from magic bytes (deflate/brotli have no signature — pass the algo explicitly)")
            })?;
            do_decompress(algo, data)
        },
    );

    // compression::decompress(algo, blob) -> Blob
    let _ = module.set_native_fn(
        "decompress",
        |algo: &str, data: Blob| -> Result<Blob, Box<EvalAltResult>> {
            let algo = compression::parse_algo(algo).map_err(|e| err(e.to_string()))?;
            do_decompress(algo, data)
        },
    );

    // compression::list() -> Array of Maps describing each algorithm.
    let _ = module.set_native_fn("list", || -> Result<Array, Box<EvalAltResult>> {
        let mut out = Array::new();
        for algo in Algo::ALL {
            let mut m = Map::new();
            m.insert("canonical".into(), algo.canonical().to_string().into());
            let aliases: Array = algo
                .aliases()
                .iter()
                .map(|a| Dynamic::from(a.to_string()))
                .collect();
            m.insert("aliases".into(), aliases.into());
            let (min, max) = algo.level_range();
            m.insert("level_min".into(), (min as i64).into());
            m.insert("level_max".into(), (max as i64).into());
            m.insert(
                "default_level".into(),
                (algo.default_level() as i64).into(),
            );
            m.insert("levelless".into(), algo.is_levelless().into());
            let magic = match algo.magic() {
                Some(bytes) => Dynamic::from(
                    bytes
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>(),
                ),
                None => Dynamic::UNIT,
            };
            m.insert("magic".into(), magic);
            out.push(Dynamic::from(m));
        }
        Ok(out)
    });

    // compression::detect(blob) -> String | ()
    let _ = module.set_native_fn(
        "detect",
        |data: Blob| -> Result<Dynamic, Box<EvalAltResult>> {
            Ok(match compression::detect_from_magic(&data) {
                Some(a) => Dynamic::from(a.canonical().to_string()),
                None => Dynamic::UNIT,
            })
        },
    );

    engine.register_static_module("compression", module.into());
}

fn do_compress(algo: Algo, level: u32, data: Blob) -> Result<Blob, Box<EvalAltResult>> {
    let source: Box<dyn std::io::Read> = Box::new(Cursor::new(data));
    let mut out: Vec<u8> = Vec::new();
    compression::compress(algo, level, source, &mut out).map_err(|e| err(e.to_string()))?;
    Ok(out)
}

fn do_decompress(algo: Algo, data: Blob) -> Result<Blob, Box<EvalAltResult>> {
    let source: Box<dyn std::io::Read> = Box::new(Cursor::new(data));
    let mut out: Vec<u8> = Vec::new();
    compression::decompress(algo, source, &mut out).map_err(|e| err(e.to_string()))?;
    Ok(out)
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
    fn round_trip_gzip_via_script() {
        let e = engine();
        let script = r#"
let payload = blob();
for b in "hello from recon".to_blob() { payload.push(b); }
let gz = compression::compress("gzip", payload);
let back = compression::decompress("gzip", gz);
back.len()
"#;
        let _ = e;
        // The simpler smoke: build a payload and round-trip, compare length.
        // Rhai doesn't have a direct "blob from string" literal; rely on
        // `to_blob()` method on Rhai strings.
        let e = engine();
        let n: i64 = e
            .eval(
                r#"
let p = "hello from recon".to_blob();
let gz = compression::compress("gzip", p);
let back = compression::decompress("gzip", gz);
back.len()
"#,
            )
            .expect("eval");
        assert_eq!(n, "hello from recon".len() as i64);
    }

    #[test]
    fn round_trip_all_nine_algos() {
        let e = engine();
        // Run separate eval per algo so a single failure is clearly labelled.
        for algo in [
            "gzip", "deflate", "zstd", "brotli", "bzip2", "lz4", "xz", "snappy", "zlib",
        ] {
            let script = format!(
                r#"
let p = "the quick brown fox".to_blob();
let c = compression::compress("{algo}", p);
let b = compression::decompress("{algo}", c);
b == "the quick brown fox".to_blob()
"#
            );
            let ok: bool = e.eval(&script).unwrap_or_else(|e| panic!("{algo}: {e}"));
            assert!(ok, "{algo} round-trip failed");
        }
    }

    #[test]
    fn auto_detect_decompress() {
        let e = engine();
        let script = r#"
let p = "payload".to_blob();
let gz = compression::compress("gzip", p);
let back = compression::decompress(gz);     // no algo — auto-detect
back == "payload".to_blob()
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn auto_detect_signatureless_throws() {
        let e = engine();
        let script = r#"
let p = "payload".to_blob();
let raw = compression::compress("deflate", p);  // no magic bytes
compression::decompress(raw)
"#;
        let res: Result<Blob, _> = e.eval(script);
        assert!(res.is_err(), "expected error, got {res:?}");
    }

    #[test]
    fn levelless_algo_rejects_level() {
        let e = engine();
        let res: Result<Blob, _> = e.eval(
            r#"compression::compress("lz4", "x".to_blob(), 5)"#,
        );
        assert!(res.is_err());
    }

    #[test]
    fn list_returns_all_algorithms() {
        let e = engine();
        let arr: Array = e.eval("compression::list()").expect("eval");
        assert_eq!(arr.len(), 9);
        // Spot-check one entry shape.
        let first = arr[0].clone().try_cast::<Map>().unwrap();
        assert!(first.contains_key("canonical"));
        assert!(first.contains_key("aliases"));
        assert!(first.contains_key("level_min"));
    }

    #[test]
    fn detect_returns_algo_name() {
        let e = engine();
        let name: String = e
            .eval(r#"compression::detect(compression::compress("gzip", "x".to_blob()))"#)
            .expect("eval");
        assert_eq!(name, "gzip");
    }

    #[test]
    fn word_form_level() {
        let e = engine();
        let ok: bool = e
            .eval(
                r#"
let p = "payload".to_blob();
let c = compression::compress("gzip", p, "best");
let b = compression::decompress("gzip", c);
b == "payload".to_blob()
"#,
            )
            .expect("eval");
        assert!(ok);
    }
}
