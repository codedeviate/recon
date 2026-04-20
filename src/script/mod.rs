//! Rhai scripting engine for `recon --script PATH`.
//!
//! Loads a `.rhai` file, registers all probe bindings (`http`, `tcp`, `dns`,
//! `tls`, …) plus helpers (`sleep_ms`, `env`, `assert`, …), and executes it.
//! The script's `return N` (integer) becomes the process exit code; uncaught
//! exceptions map to non-zero exits via `ProtocolExitCode` where applicable.

use crate::cli::Args;

pub mod engine;

/// Entry point from `main.rs`. Returns the process exit code.
pub fn run(args: &Args) -> i32 {
    let path = match &args.script {
        Some(p) => p.clone(),
        None => {
            eprintln!("error: --script requires a path");
            return 1;
        }
    };
    engine::run_file(&path, args)
}
