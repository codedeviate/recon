//! Meta-command parser and dispatcher. Meta-commands start with `:`
//! (e.g. `:help`, `:load foo.rhai`, `:set autoprint off`). The
//! dispatcher mutates the shared `ReplState`.
//!
//! Parse rule: split off the first whitespace-delimited token (the
//! command, with the leading `:` stripped). The remainder is passed
//! verbatim to the handler so `:set header X-Foo: bar` works without
//! re-quoting.

use super::ReplState;

/// Outcome of a meta-command. `Quit` ends the REPL. `Paste` signals
/// the main loop to enter multi-line paste mode.
pub enum Outcome {
    Continue,
    Quit,
    Paste,
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
        "load" => { cmd_load(state, rest); Outcome::Continue }
        "run" => { cmd_run(state, rest); Outcome::Continue }
        "set" => { cmd_set(state, rest); Outcome::Continue }
        "paste" => Outcome::Paste,
        "save" => { cmd_save(state, rest); Outcome::Continue }
        "history" => { cmd_history(state, rest); Outcome::Continue }
        "edit" => { cmd_edit(state); Outcome::Continue }
        "time" => { cmd_time(state, rest); Outcome::Continue }
        bang if bang.starts_with('!') => {
            cmd_rerun(state, &bang[1..]);
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

fn cmd_load(state: &mut ReplState, rest: &str) {
    let path = match resolve_script_path(rest) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("error: {msg}");
            return;
        }
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {e}", path.display());
            return;
        }
    };
    super::eval_and_print_load(state, &source);
}

fn cmd_run(state: &ReplState, rest: &str) {
    let path = match resolve_script_path(rest) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("error: {msg}");
            return;
        }
    };
    let result = super::run_script_isolated(&path, &state.defaults);
    match result {
        Ok(value) => {
            if let Some(s) = super::print::format(&value) {
                println!("{s}");
            }
        }
        Err(e) => eprintln!("error: {e}"),
    }
}

fn cmd_set(state: &mut ReplState, rest: &str) {
    let (key, value) = match rest.split_once(char::is_whitespace) {
        Some((k, v)) => (k, v.trim()),
        None => {
            eprintln!(
                "error: usage :set <key> <value>. Keys: method, header, timeout, user-agent, autoprint"
            );
            return;
        }
    };
    match key {
        "autoprint" => match value {
            "on" | "true" | "1" => {
                state.autoprint = true;
                println!("autoprint = on");
            }
            "off" | "false" | "0" => {
                state.autoprint = false;
                println!("autoprint = off");
            }
            _ => eprintln!("error: :set autoprint on|off"),
        },
        "method" => {
            state.defaults.method = Some(value.to_uppercase());
            rebuild_flags(state);
            println!("method = {}", value.to_uppercase());
        }
        "header" => {
            if !value.contains(':') {
                eprintln!("error: :set header expects 'Name: value', got '{value}'");
                return;
            }
            state.defaults.headers.push(value.to_string());
            rebuild_flags(state);
            println!("header added: {value}");
        }
        "timeout" => match value.parse::<u64>() {
            Ok(n) => {
                state.defaults.connect_timeout = n;
                rebuild_flags(state);
                println!("timeout = {n}s");
            }
            Err(_) => eprintln!("error: :set timeout expects a number of seconds"),
        },
        "user-agent" => {
            state.defaults.user_agent = Some(value.to_string());
            rebuild_flags(state);
            println!("user-agent = {value}");
        }
        other => {
            eprintln!(
                "error: unknown key '{other}'. Allowed: method, header, timeout, user-agent, autoprint"
            );
        }
    }
}

fn rebuild_flags(state: &mut ReplState) {
    // `flags` was pushed as a constant (ReadOnly) so we cannot use
    // `set_value` — that would panic. Instead, push a new constant with
    // the same name. Rhai's Scope searches from the most-recent entry
    // backwards, so the new binding shadows the old one transparently.
    let new_flags = super::build_flags_from_defaults(&state.defaults);
    state.scope.push_constant("flags", new_flags);
}

fn cmd_save(state: &ReplState, rest: &str) {
    if rest.is_empty() {
        eprintln!("error: usage :save <path>");
        return;
    }
    let header = format!(
        "// recon REPL session — {} epoch s\n\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let body = state.history.join("\n");
    if let Err(e) = std::fs::write(rest, format!("{header}{body}\n")) {
        eprintln!("error: write {rest}: {e}");
    } else {
        println!("saved {} entries to {rest}", state.history.len());
    }
}

fn cmd_history(state: &ReplState, rest: &str) {
    let n: usize = if rest.is_empty() {
        20
    } else {
        rest.parse().unwrap_or(20)
    };
    let start = state.history.len().saturating_sub(n);
    for (i, line) in state.history.iter().enumerate().skip(start) {
        println!("  {:4}  {}", i + 1, line.replace('\n', " ⏎ "));
    }
    if state.history.is_empty() {
        println!("(empty)");
    }
}

fn cmd_rerun(state: &mut ReplState, n_str: &str) {
    let n: usize = match n_str.parse() {
        Ok(n) if n >= 1 => n,
        _ => {
            eprintln!("error: :!N expects a 1-based index, got '{n_str}'");
            return;
        }
    };
    let idx = n - 1;
    let Some(line) = state.history.get(idx).cloned() else {
        eprintln!("error: no history entry {n} (have {})", state.history.len());
        return;
    };
    println!("(rerun) {line}");
    super::eval_and_print_load(state, &line);
}

fn resolve_script_path(rest: &str) -> Result<std::path::PathBuf, String> {
    if rest.is_empty() {
        return Err("usage: :load <path>  or  :run <path>".into());
    }
    let literal = std::path::PathBuf::from(rest);
    if literal.exists() {
        return Ok(literal);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        let recon = home.join(".recon").join("script").join(rest);
        if recon.exists() {
            return Ok(recon);
        }
        if !rest.ends_with(".rhai") {
            let recon_rhai = home.join(".recon").join("script")
                .join(format!("{rest}.rhai"));
            if recon_rhai.exists() {
                return Ok(recon_rhai);
            }
        }
    }
    Err(format!("script not found: {rest} (also tried ~/.recon/script/{rest}[.rhai])"))
}

fn cmd_edit(state: &mut ReplState) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let tmp = std::env::temp_dir().join(format!(
        "recon-repl-{}.rhai",
        std::process::id()
    ));
    // Pre-populate with a hint comment so saving an empty file is a no-op.
    let _ = std::fs::write(&tmp, "// Type Rhai below. Save and quit your editor to evaluate.\n");

    let status = std::process::Command::new(&editor)
        .arg(&tmp)
        .status();
    match status {
        Ok(s) if s.success() => {
            let source = std::fs::read_to_string(&tmp).unwrap_or_default();
            let _ = std::fs::remove_file(&tmp);
            // If the only content is the hint comment, treat as empty.
            let trimmed: String = source
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            if trimmed.trim().is_empty() {
                println!("(empty)");
                return;
            }
            super::eval_and_print_load(state, &source);
        }
        Ok(s) => eprintln!("error: editor exited with status {s}"),
        Err(e) => eprintln!("error: could not launch '{editor}': {e}"),
    }
}

fn cmd_time(state: &mut ReplState, expr: &str) {
    if expr.is_empty() {
        eprintln!("error: usage :time <expr>");
        return;
    }
    let start = std::time::Instant::now();
    super::eval_and_print_load(state, expr);
    let elapsed = start.elapsed();
    println!("elapsed: {:.3?}", elapsed);
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
