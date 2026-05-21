//! Interactive REPL mode (`recon --repl`).
//!
//! Provides a Rhai prompt with the same bindings as `--script`. The loop,
//! meta-command parser, multi-line detection, and pretty-printer live in
//! sibling modules. Entry point: `run(&args)` returns the process exit
//! code.

mod multiline;
mod print;

use crate::cli::Args;

pub fn run(_args: &Args) -> i32 {
    eprintln!("recon REPL — :help for commands, :quit to exit");
    eprintln!("(stub — full loop arrives in Task 6)");
    0
}
