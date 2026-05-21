//! Meta-command parser and dispatcher. Meta-commands start with `:`
//! (e.g. `:help`, `:load foo.rhai`, `:set autoprint off`). The
//! dispatcher mutates the shared `ReplState`.
//!
//! Parse rule: split off the first whitespace-delimited token (the
//! command, with the leading `:` stripped). The remainder is passed
//! verbatim to the handler so `:set header X-Foo: bar` works without
//! re-quoting.

use super::ReplState;

/// Outcome of a meta-command. `Quit` ends the REPL.
pub enum Outcome {
    Continue,
    Quit,
}

/// Parse and dispatch a single meta-command line (including the leading
/// `:`). Prints output as appropriate and returns the outcome.
pub fn dispatch(line: &str, state: &mut ReplState) -> Outcome {
    let body = line.trim().strip_prefix(':').unwrap_or(line.trim());
    let (cmd, rest) = match body.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (body, ""),
    };

    match cmd {
        "quit" | "exit" => Outcome::Quit,
        "help" => {
            if rest.is_empty() {
                cheat_sheet();
            } else {
                topic_passthrough(rest);
            }
            Outcome::Continue
        }
        "vars" => {
            cmd_vars(state);
            Outcome::Continue
        }
        "fns" => {
            cmd_fns(state);
            Outcome::Continue
        }
        "reset" => {
            cmd_reset(state);
            Outcome::Continue
        }
        // Commands defined in later tasks; emit a clear "not yet" so
        // the wiring is visible even before they're implemented:
        "load" | "run" | "set" | "paste" | "save" | "history" | "edit" | "time" => {
            eprintln!("error: :{cmd} not implemented in this build (coming in Task 9-13)");
            Outcome::Continue
        }
        bang if bang.starts_with('!') => {
            eprintln!("error: :!N (history rerun) not implemented yet");
            Outcome::Continue
        }
        unknown => {
            eprintln!("error: unknown command ':{unknown}'. Try :help.");
            Outcome::Continue
        }
    }
}

fn cheat_sheet() {
    println!(
        r#"recon REPL — interactive Rhai prompt.

Meta-commands (start with ':'):
  :help              this cheat sheet
  :help <topic>      print `recon --help <topic>` content (http, jwt, ...)
  :load <path>       eval <path> in current scope (let/fn persist)
  :run <path>        eval <path> in a fresh scope (REPL state untouched)
  :paste             enter paste mode; finish with ':end' on its own line
  :set <key> <val>   mutate flags (method, header, timeout, user-agent, autoprint)
  :vars              list bound variables
  :fns               list user-defined functions
  :reset             clear bindings (keep history)
  :save <path>       write this session's inputs to <path>
  :history [N]       print last N inputs (default 20)
  :!N                re-run history entry N
  :edit              open $EDITOR for multi-line composition
  :time <expr>       evaluate <expr> and print elapsed ms
  :quit / :exit      exit REPL

Multi-line input is detected automatically (open `{{`, open `(`, open `"`).
Bare expressions print their result (autoprint on by default; :set autoprint off to disable).
Type Ctrl-C to cancel a multi-line buffer, Ctrl-D to exit."#
    );
}

fn topic_passthrough(topic: &str) {
    if !crate::help::print_topic(topic) {
        eprintln!("error: unknown help topic '{topic}'. Try `:help` for the REPL cheat sheet, or run `recon --help` for the full topic list.");
    }
}

fn cmd_vars(state: &ReplState) {
    let mut any = false;
    for (name, is_const, value) in state.scope.iter() {
        any = true;
        let tag = if is_const { "const" } else { "let  " };
        let preview = super::print::format(&value).unwrap_or_else(|| "()".into());
        println!("  {tag} {name} = {preview}");
    }
    if !any {
        println!("(no bindings)");
    }
}

fn cmd_fns(state: &ReplState) {
    let mut any = false;
    for ast in &state.user_asts {
        for f in ast.iter_functions() {
            any = true;
            println!("  fn {}/{}", f.name, f.params.len());
        }
    }
    if !any {
        println!("(no user-defined functions)");
    }
}

fn cmd_reset(state: &mut ReplState) {
    let args_snapshot = state.scope.get_value::<rhai::Array>("args");
    let flags_snapshot = state.scope.get_value::<rhai::Map>("flags");
    let path_snapshot = state.scope.get_value::<String>("script_path");
    let dir_snapshot = state.scope.get_value::<String>("script_dir");
    let name_snapshot = state.scope.get_value::<String>("script_name");

    let mut fresh = rhai::Scope::new();
    if let Some(v) = args_snapshot {
        fresh.push_constant("args", v);
    }
    if let Some(v) = flags_snapshot {
        fresh.push_constant("flags", v);
    }
    if let Some(v) = path_snapshot {
        fresh.push_constant("script_path", v);
    }
    if let Some(v) = dir_snapshot {
        fresh.push_constant("script_dir", v);
    }
    if let Some(v) = name_snapshot {
        fresh.push_constant("script_name", v);
    }

    state.scope = fresh;
    state.user_asts.clear();
    println!("(scope cleared)");
}

#[cfg(test)]
mod tests {
    // Parser-only tests; full ReplState behaviour is covered by the
    // integration test in tests/repl_it.rs.

    fn split(input: &str) -> (String, String) {
        let body = input.trim().strip_prefix(':').unwrap_or(input.trim());
        let (cmd, rest) = match body.split_once(char::is_whitespace) {
            Some((c, r)) => (c.to_string(), r.trim().to_string()),
            None => (body.to_string(), String::new()),
        };
        (cmd, rest)
    }

    #[test]
    fn splits_command_and_remainder() {
        assert_eq!(split(":help"), ("help".into(), "".into()));
        assert_eq!(split(":help http"), ("help".into(), "http".into()));
        assert_eq!(
            split(":set header X-Foo: bar"),
            ("set".into(), "header X-Foo: bar".into())
        );
        assert_eq!(split(":quit"), ("quit".into(), "".into()));
    }

    #[test]
    fn handles_leading_whitespace() {
        assert_eq!(split("  :vars  "), ("vars".into(), "".into()));
    }
}
