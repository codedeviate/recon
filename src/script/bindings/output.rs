//! Raw-print helpers: `print_raw`, `eprint`, `eprint_raw`, `flush`.
//!
//! Rhai's built-in `print()` always appends a newline and writes via the
//! engine's debug callback. These bindings write directly to stdout /
//! stderr, giving scripts byte-precise control when building line
//! protocols, progress bars, or terminal UIs.

use rhai::Engine;
use std::io::Write;

pub fn register(engine: &mut Engine) {
    engine.register_fn("print_raw", |s: &str| {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.flush();
    });

    engine.register_fn("print_raw", |b: rhai::Blob| {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(&b);
        let _ = handle.flush();
    });

    engine.register_fn("eprint", |s: &str| {
        eprintln!("{s}");
    });

    engine.register_fn("eprint_raw", |s: &str| {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.flush();
    });

    engine.register_fn("eprint_raw", |b: rhai::Blob| {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = handle.write_all(&b);
        let _ = handle.flush();
    });

    engine.register_fn("flush", || {
        let _ = std::io::stdout().flush();
    });
}
