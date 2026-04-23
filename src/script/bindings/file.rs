//! File-I/O script bindings.
//!
//! Two styles coexist:
//!
//! 1. **Whole-file convenience** — `file_read(path)` returns the full
//!    contents as a `Blob`; `file_write_all(path, blob)` overwrites;
//!    `file_append_all(path, blob)` appends; plus `file_exists`,
//!    `file_size`, `file_delete`.
//! 2. **Streaming handles** — `file_open(path, mode)` returns a
//!    `FileHandle`, a newtype around `Arc<Mutex<File>>`. Method-style
//!    helpers (`read`, `read_all`, `write`, `seek`, `tell`, `flush`,
//!    `close`) let scripts stream bytes and seek without loading the
//!    entire file into memory.
//!
//! `Arc<Mutex<File>>` is deliberate — it keeps the handles `Send + Sync`
//! so they survive the planned rhai `sync`-feature flip (0.56.0) without
//! a refactor.

use crate::script::convert::err;
use crate::source::{resolve_file_url, SourceKind};
use rhai::{Blob, Engine, EvalAltResult};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct FileHandle {
    inner: Arc<Mutex<File>>,
}

impl FileHandle {
    fn new(file: File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(file)),
        }
    }
}

fn resolve_path(raw: &str) -> Result<PathBuf, Box<EvalAltResult>> {
    if raw.starts_with("file://") {
        match resolve_file_url(raw).map_err(|e| err(e.to_string()))? {
            SourceKind::File(p) => Ok(p),
            _ => Err(err("file path: unexpected non-file source")),
        }
    } else {
        Ok(PathBuf::from(raw))
    }
}

pub fn register(engine: &mut Engine) {
    // ── Whole-file convenience ──────────────────────────────────────────

    engine.register_fn("file_read", |path: &str| -> Result<Blob, Box<EvalAltResult>> {
        let target = resolve_path(path)?;
        std::fs::read(&target).map_err(|e| {
            err(format!(
                "file_read: could not read '{}': {e}",
                target.display()
            ))
        })
    });

    engine.register_fn(
        "file_write_all",
        |path: &str, data: Blob| -> Result<i64, Box<EvalAltResult>> {
            let target = resolve_path(path)?;
            std::fs::write(&target, &data).map_err(|e| {
                err(format!(
                    "file_write_all: could not write '{}': {e}",
                    target.display()
                ))
            })?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn(
        "file_write_all",
        |path: &str, data: &str| -> Result<i64, Box<EvalAltResult>> {
            let target = resolve_path(path)?;
            std::fs::write(&target, data.as_bytes()).map_err(|e| {
                err(format!(
                    "file_write_all: could not write '{}': {e}",
                    target.display()
                ))
            })?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn(
        "file_append_all",
        |path: &str, data: Blob| -> Result<i64, Box<EvalAltResult>> {
            let target = resolve_path(path)?;
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&target)
                .map_err(|e| {
                    err(format!(
                        "file_append_all: could not open '{}': {e}",
                        target.display()
                    ))
                })?;
            f.write_all(&data).map_err(|e| {
                err(format!(
                    "file_append_all: could not write '{}': {e}",
                    target.display()
                ))
            })?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn(
        "file_append_all",
        |path: &str, data: &str| -> Result<i64, Box<EvalAltResult>> {
            let target = resolve_path(path)?;
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&target)
                .map_err(|e| {
                    err(format!(
                        "file_append_all: could not open '{}': {e}",
                        target.display()
                    ))
                })?;
            f.write_all(data.as_bytes()).map_err(|e| {
                err(format!(
                    "file_append_all: could not write '{}': {e}",
                    target.display()
                ))
            })?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn("file_exists", |path: &str| -> bool {
        match resolve_path(path) {
            Ok(p) => p.exists(),
            Err(_) => false,
        }
    });

    engine.register_fn("file_size", |path: &str| -> Result<i64, Box<EvalAltResult>> {
        let target = resolve_path(path)?;
        let meta = std::fs::metadata(&target).map_err(|e| {
            err(format!(
                "file_size: could not stat '{}': {e}",
                target.display()
            ))
        })?;
        Ok(meta.len() as i64)
    });

    engine.register_fn("file_delete", |path: &str| -> Result<(), Box<EvalAltResult>> {
        let target = resolve_path(path)?;
        std::fs::remove_file(&target).map_err(|e| {
            err(format!(
                "file_delete: could not remove '{}': {e}",
                target.display()
            ))
        })
    });

    // ── Streaming handles ──────────────────────────────────────────────

    engine.register_type_with_name::<FileHandle>("FileHandle");

    engine.register_fn(
        "file_open",
        |path: &str, mode: &str| -> Result<FileHandle, Box<EvalAltResult>> {
            let target = resolve_path(path)?;
            let mut opts = OpenOptions::new();
            match mode {
                "r" => {
                    opts.read(true);
                }
                "w" => {
                    opts.write(true).create(true).truncate(true);
                }
                "rw" | "r+" => {
                    opts.read(true).write(true);
                }
                "rwc" | "w+" => {
                    opts.read(true).write(true).create(true).truncate(true);
                }
                "a" => {
                    opts.append(true).create(true);
                }
                "ra" => {
                    opts.read(true).append(true).create(true);
                }
                other => {
                    return Err(err(format!(
                        "file_open: unknown mode '{other}' (want r|w|rw|rwc|a|ra)"
                    )))
                }
            }
            let file = opts.open(&target).map_err(|e| {
                err(format!(
                    "file_open: could not open '{}': {e}",
                    target.display()
                ))
            })?;
            Ok(FileHandle::new(file))
        },
    );

    engine.register_fn(
        "file_read",
        |h: &mut FileHandle, n: i64| -> Result<Blob, Box<EvalAltResult>> {
            if n < 0 {
                return Err(err("file_read: n must be non-negative"));
            }
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_read: mutex poisoned"))?;
            let mut buf = vec![0u8; n as usize];
            let read = guard
                .read(&mut buf)
                .map_err(|e| err(format!("file_read: {e}")))?;
            buf.truncate(read);
            Ok(buf)
        },
    );

    engine.register_fn(
        "file_read_all",
        |h: &mut FileHandle| -> Result<Blob, Box<EvalAltResult>> {
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_read_all: mutex poisoned"))?;
            let mut buf = Vec::new();
            guard
                .read_to_end(&mut buf)
                .map_err(|e| err(format!("file_read_all: {e}")))?;
            Ok(buf)
        },
    );

    engine.register_fn(
        "file_write",
        |h: &mut FileHandle, data: Blob| -> Result<i64, Box<EvalAltResult>> {
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_write: mutex poisoned"))?;
            guard
                .write_all(&data)
                .map_err(|e| err(format!("file_write: {e}")))?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn(
        "file_write",
        |h: &mut FileHandle, data: &str| -> Result<i64, Box<EvalAltResult>> {
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_write: mutex poisoned"))?;
            guard
                .write_all(data.as_bytes())
                .map_err(|e| err(format!("file_write: {e}")))?;
            Ok(data.len() as i64)
        },
    );

    engine.register_fn(
        "file_seek",
        |h: &mut FileHandle, pos: i64, whence: &str| -> Result<i64, Box<EvalAltResult>> {
            let to = match whence {
                "start" => SeekFrom::Start(pos.max(0) as u64),
                "cur" | "current" => SeekFrom::Current(pos),
                "end" => SeekFrom::End(pos),
                other => {
                    return Err(err(format!(
                        "file_seek: unknown whence '{other}' (want start|cur|end)"
                    )))
                }
            };
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_seek: mutex poisoned"))?;
            guard
                .seek(to)
                .map(|p| p as i64)
                .map_err(|e| err(format!("file_seek: {e}")))
        },
    );

    engine.register_fn(
        "file_tell",
        |h: &mut FileHandle| -> Result<i64, Box<EvalAltResult>> {
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_tell: mutex poisoned"))?;
            guard
                .stream_position()
                .map(|p| p as i64)
                .map_err(|e| err(format!("file_tell: {e}")))
        },
    );

    engine.register_fn(
        "file_flush",
        |h: &mut FileHandle| -> Result<(), Box<EvalAltResult>> {
            let mut guard = h
                .inner
                .lock()
                .map_err(|_| err("file_flush: mutex poisoned"))?;
            guard
                .flush()
                .map_err(|e| err(format!("file_flush: {e}")))
        },
    );

    engine.register_fn("file_close", |_h: FileHandle| {
        // Dropping the last Arc closes the underlying file.
    });
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

    #[test]
    fn write_all_and_read_back() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let e = engine();
        let script = format!(
            r#"
let n = file_write_all("{path}", "round-trip");
assert(n == 10, "bytes written");
let b = file_read("{path}");
b
"#
        );
        let b: Blob = e.eval(&script).expect("eval");
        assert_eq!(b.as_slice(), b"round-trip");
    }

    #[test]
    fn append_concatenates() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let e = engine();
        let script = format!(
            r#"
file_write_all("{path}", "part1");
file_append_all("{path}", "part2");
file_read("{path}").len()
"#
        );
        let n: i64 = e.eval(&script).expect("eval");
        assert_eq!(n, 10);
    }

    #[test]
    fn exists_size_delete_round_trip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let e = engine();
        let script = format!(
            r#"
file_write_all("{path}", "abc");
if !file_exists("{path}") {{ return -1; }}
let s = file_size("{path}");
file_delete("{path}");
if file_exists("{path}") {{ return -2; }}
s
"#
        );
        let n: i64 = e.eval(&script).expect("eval");
        assert_eq!(n, 3);
    }

    #[test]
    fn streaming_handle_read_write_seek() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let e = engine();
        let script = format!(
            r#"
let h = file_open("{path}", "rwc");
file_write(h, "abcdef");
file_seek(h, 0, "start");
let p1 = file_read(h, 3);
let t = file_tell(h);
file_close(h);
#{{ p1: p1.len(), tell: t }}
"#
        );
        let m: rhai::Map = e.eval(&script).expect("eval");
        assert_eq!(m.get("p1").unwrap().as_int().unwrap(), 3);
        assert_eq!(m.get("tell").unwrap().as_int().unwrap(), 3);
    }

    #[test]
    fn unknown_mode_errors() {
        let e = engine();
        let res: Result<FileHandle, _> = e.eval(r#"file_open("/tmp/whatever", "q")"#);
        assert!(res.is_err());
    }
}
