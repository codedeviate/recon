//! Interactive REPL mode (`recon --repl`).
//!
//! Persists one `rhai::Engine`, one `rhai::Scope`, and a `Vec<AST>` of
//! user-defined-function chunks across lines. `let` bindings persist in
//! the scope; `fn` definitions persist via AST accumulation (merged
//! into each newly compiled AST before eval).

mod meta;
mod multiline;
mod print;

use crate::cli::Args;
use crate::script::{bindings, defaults::ScriptDefaults, engine::build_engine};
use rhai::{Dynamic, Scope, AST};
use rustyline::error::ReadlineError;
use rustyline::{Config, DefaultEditor};
use std::path::PathBuf;

pub(super) struct ReplState {
    pub(super) engine: rhai::Engine,
    pub(super) scope: Scope<'static>,
    pub(super) user_asts: Vec<AST>,
    pub(super) autoprint: bool,
    pub(super) history: Vec<String>,
    #[allow(dead_code)] // used by :set in Task 10
    pub(super) defaults: ScriptDefaults,
    #[allow(dead_code)] // used by :save in Task 11
    pub(super) history_path: PathBuf,
}

pub fn run(args: &Args) -> i32 {
    let defaults = ScriptDefaults::from_args(args);
    let mut engine = build_engine(&defaults);
    bindings::thread::register_repl_stub(&mut engine);

    let mut scope = Scope::new();
    scope.push_constant("args", bindings::cli::build_args_array(args));
    scope.push_constant("flags", bindings::cli::build_flags_map(args));
    scope.push_constant("script_path", "<repl>".to_string());
    scope.push_constant(
        "script_dir",
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    scope.push_constant("script_name", "repl".to_string());

    let history_path = args
        .repl_history
        .clone()
        .unwrap_or_else(default_history_path);

    let mut state = ReplState {
        engine,
        scope,
        user_asts: Vec::new(),
        autoprint: true,
        history: Vec::new(),
        defaults,
        history_path: history_path.clone(),
    };

    let rl_config = Config::builder().auto_add_history(true).build();
    let mut rl = match DefaultEditor::with_config(rl_config) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: could not initialize line editor: {e}");
            return 1;
        }
    };
    if history_path.exists() {
        let _ = rl.load_history(&history_path);
    }

    eprintln!("recon REPL — :help for commands, :quit to exit");

    let mut buffer = String::new();
    loop {
        let prompt = if buffer.is_empty() { ">>> " } else { "... " };
        match rl.readline(prompt) {
            Ok(line) => {
                // Empty buffer + meta-command → dispatch and continue.
                if buffer.is_empty() && line.trim_start().starts_with(':') {
                    match meta::dispatch(&line, &mut state) {
                        meta::Outcome::Continue => continue,
                        meta::Outcome::Quit => break,
                    }
                }
                if !buffer.is_empty() {
                    buffer.push('\n');
                }
                buffer.push_str(&line);

                use multiline::Status;
                match multiline::classify(&state.engine, &buffer) {
                    Status::NeedMore => continue,
                    Status::Syntax(msg) => {
                        eprintln!("error: {msg}");
                        buffer.clear();
                        continue;
                    }
                    Status::Complete => {
                        let source = std::mem::take(&mut buffer);
                        eval_and_print(&mut state, &source);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                buffer.clear();
                eprintln!("^C");
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    }

    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(&history_path);
    0
}

fn eval_and_print(state: &mut ReplState, source: &str) {
    let mut ast = match state
        .engine
        .compile_into_self_contained(&state.scope, source)
    {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    // Merge accumulated user-defined fns from earlier lines so they
    // remain callable. `combine` is last-wins on duplicate names.
    for prev in &state.user_asts {
        ast.combine(prev.clone());
    }
    match state
        .engine
        .eval_ast_with_scope::<Dynamic>(&mut state.scope, &ast)
    {
        Ok(value) => {
            if ast.iter_functions().count() > 0 {
                // Persist only the function-bearing AST (drop the
                // statements that already ran via eval).
                state.user_asts.push(ast.clone_functions_only());
            }
            state.history.push(source.to_string());
            if state.autoprint {
                if let Some(s) = print::format(&value) {
                    println!("{s}");
                }
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
        }
    }
}

fn default_history_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".recon")
        .join("repl_history")
}
