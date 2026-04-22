//! `archive::*` static module — Rhai bindings for the archive tools
//! shipped as `--archive` / `--extract` in 0.35.0.
//!
//! Sources for `create` accept an Array of path strings; both functions
//! return the file count (i64). `detect` returns the format name ("zip",
//! "tar", "tar.gz", …) or `()` when unrecognised.

use crate::archive::{self, Format};
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Module};
use std::path::{Path, PathBuf};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    let _ = module.set_native_fn(
        "create",
        |dest: &str, sources: Array| -> Result<i64, Box<EvalAltResult>> {
            let dest_path = PathBuf::from(dest);
            let fmt = archive::detect_from_path(&dest_path).ok_or_else(|| {
                err(format!(
                    "archive: can't infer format from '{dest}' (expected .zip / .tar / .tar.gz / .tar.xz / .tar.bz2 or alias)"
                ))
            })?;
            let src_paths: Vec<PathBuf> = sources
                .into_iter()
                .map(|v| {
                    if v.is_string() {
                        PathBuf::from(v.into_string().unwrap_or_default())
                    } else {
                        PathBuf::from(v.to_string())
                    }
                })
                .collect();
            if src_paths.is_empty() {
                return Err(err("archive::create: sources array must not be empty"));
            }
            let count = archive::create(&dest_path, &src_paths, fmt)
                .map_err(|e| err(e.to_string()))?;
            Ok(count as i64)
        },
    );

    let _ = module.set_native_fn(
        "extract",
        |src: &str, dest: &str| -> Result<i64, Box<EvalAltResult>> {
            let src_path = PathBuf::from(src);
            let dest_path = PathBuf::from(dest);
            let fmt = detect_extract_format(&src_path)?;
            std::fs::create_dir_all(&dest_path).map_err(|e| {
                err(format!(
                    "archive::extract: create dest {dest}: {e}"
                ))
            })?;
            let count = archive::extract(&src_path, &dest_path, fmt)
                .map_err(|e| err(e.to_string()))?;
            Ok(count as i64)
        },
    );

    let _ = module.set_native_fn(
        "detect",
        |path: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            Ok(match archive::detect_from_path(Path::new(path)) {
                Some(f) => Dynamic::from(f.label().to_string()),
                None => Dynamic::UNIT,
            })
        },
    );

    engine.register_static_module("archive", module.into());
}

/// Use extension first, then magic-byte sniffing (mirrors the CLI path).
fn detect_extract_format(src: &Path) -> Result<Format, Box<EvalAltResult>> {
    if let Some(f) = archive::detect_from_path(src) {
        return Ok(f);
    }
    let mut head = [0u8; 512];
    let mut f = std::fs::File::open(src)
        .map_err(|e| err(format!("archive::extract: open {}: {e}", src.display())))?;
    use std::io::Read;
    let n = f.read(&mut head).unwrap_or(0);
    archive::detect_from_magic(&head[..n]).ok_or_else(|| {
        err(format!(
            "archive::extract: can't infer format of '{}' from extension or magic bytes",
            src.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn detect_returns_format_labels() {
        let e = engine();
        let s: String = e.eval(r#"archive::detect("foo.tar.gz")"#).unwrap();
        assert_eq!(s, "tar.gz");
        let s: String = e.eval(r#"archive::detect("foo.tgz")"#).unwrap();
        assert_eq!(s, "tar.gz");
        let s: String = e.eval(r#"archive::detect("foo.zip")"#).unwrap();
        assert_eq!(s, "zip");
        // Unknown extension — returns ().
        let d: Dynamic = e.eval(r#"archive::detect("foo.unknown")"#).unwrap();
        assert!(d.is_unit());
    }

    #[test]
    fn zip_round_trip_via_script() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("hello.txt");
        std::fs::write(&src, b"contents").unwrap();
        let dest = dir.path().join("out.zip");
        let unpack = dir.path().join("unpack");

        let e = engine();
        let script = format!(
            r#"
let n = archive::create("{dest}", ["{src}"]);
let m = archive::extract("{dest}", "{unpack}");
[n, m]
"#,
            dest = dest.display(),
            src = src.display(),
            unpack = unpack.display(),
        );
        let arr: Array = e.eval(&script).expect("eval");
        assert_eq!(arr[0].as_int().unwrap(), 1);
        assert_eq!(arr[1].as_int().unwrap(), 1);
        assert_eq!(
            std::fs::read(unpack.join("hello.txt")).unwrap(),
            b"contents"
        );
    }

    #[test]
    fn empty_sources_throws() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("out.zip");
        let e = engine();
        let script = format!(r#"archive::create("{}", [])"#, dest.display());
        let res: Result<i64, _> = e.eval(&script);
        assert!(res.is_err());
    }

    #[test]
    fn unknown_format_throws_on_create() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("out.weird");
        let e = engine();
        let script = format!(r#"archive::create("{}", ["foo"])"#, dest.display());
        let res: Result<i64, _> = e.eval(&script);
        assert!(res.is_err());
    }
}
