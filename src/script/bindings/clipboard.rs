//! `clipboard::*` static module — Rhai bindings for read/write access
//! to the system clipboard via the `arboard` crate.
//!
//! - `clipboard::get() -> string` — current clipboard text.
//! - `clipboard::set(text)` — replace clipboard with `text`.
//!
//! Errors (no display server, sandboxed env, non-text clipboard content)
//! raise into Rhai errors via the standard `convert::err` helper.

use crate::script::convert::err;
use rhai::{Engine, EvalAltResult, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    let _ = module.set_native_fn(
        "get",
        || -> Result<String, Box<EvalAltResult>> {
            crate::clipboard::read_text().map_err(|e| err(e.to_string()))
        },
    );

    let _ = module.set_native_fn(
        "set",
        |text: &str| -> Result<(), Box<EvalAltResult>> {
            crate::clipboard::write_text(text).map_err(|e| err(e.to_string()))
        },
    );

    engine.register_static_module("clipboard", module.into());
}
