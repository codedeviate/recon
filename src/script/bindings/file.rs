//! `file_read(path)` script binding.
//!
//! Reads a local file path OR a `file://` URL and returns its contents
//! as a Rhai `Blob` (Vec<u8>). Use Rhai's built-in conversions to turn
//! it into a UTF-8 string (`to_string`) or hex/base64 (via helpers).

use crate::script::convert::err;
use crate::source::{resolve_file_url, SourceKind};
use rhai::{Blob, Engine, EvalAltResult};

pub fn register(engine: &mut Engine) {
    engine.register_fn(
        "file_read",
        |path: &str| -> Result<Blob, Box<EvalAltResult>> {
            let target = if path.starts_with("file://") {
                match resolve_file_url(path).map_err(|e| err(e.to_string()))? {
                    SourceKind::File(p) => p,
                    _ => return Err(err("file_read: unexpected non-file source")),
                }
            } else {
                std::path::PathBuf::from(path)
            };
            std::fs::read(&target).map_err(|e| {
                err(format!(
                    "file_read: could not read '{}': {e}",
                    target.display()
                ))
            })
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn reads_local_path_as_blob() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello bytes").unwrap();
        let path = f.path().to_str().unwrap().to_string();
        let e = engine();
        let script = format!(r#"file_read("{path}").len()"#);
        let n: i64 = e.eval(&script).expect("eval");
        assert_eq!(n, 11);
    }

    #[test]
    fn reads_file_url() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"from file url").unwrap();
        let url = format!("file://{}", f.path().display());
        let e = engine();
        let script = format!(r#"file_read("{url}").len()"#);
        let n: i64 = e.eval(&script).expect("eval");
        assert_eq!(n, 13);
    }

    #[test]
    fn missing_file_throws() {
        let e = engine();
        let res: Result<Blob, _> = e.eval(r#"file_read("/nonexistent/xyz/abc/file.txt")"#);
        assert!(res.is_err());
    }
}
