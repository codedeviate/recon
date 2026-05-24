//! `shell(cmd, [opts])` and `shell_stream(cmd, callback, [opts])` —
//! run a subprocess from a script.
//!
//! `shell` is the blocking, capture-everything form. Returns a Map with
//! `stdout`, `stderr`, `exit_code`, `success`. Suits the "run one
//! command, parse its output" pattern that scripts already use via
//! string-handling helpers.
//!
//! `shell_stream` is the streaming form: a Rhai function-pointer
//! callback fires once per line as the subprocess writes it. Returns
//! the exit code when the process is done. Built for live dashboards
//! and progress UIs — the upcoming TUI pane primitive routes
//! subprocess output to a pane by handing this function a pane-write
//! callback.
//!
//! Input shapes:
//!   - String input runs through the platform shell: `sh -c <string>`
//!     on Unix, `cmd /C <string>` on Windows. So pipes, globs,
//!     redirects, and `&&` chains work out of the box.
//!   - Array input is treated as a literal argv — `shell(["git",
//!     "log", "--oneline"])`. No shell layer, no quoting surprises.
//!
//! Opts map (all keys optional):
//!   - `cwd`: working directory (default: inherit).
//!   - `env`: extra env vars layered on top of the parent env.
//!   - `env_clear`: bool, drop the parent env entirely first.
//!   - `timeout_ms`: kill the child after N ms; raises an error.
//!   - `merge_stderr`: blocking form only — fold stderr into stdout.
//!     Streaming form always merges.

use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, FnPtr, Map, NativeCallContext};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub fn register(engine: &mut Engine) {
    // ---- blocking shell() forms -------------------------------------

    // shell(cmd_string)
    engine.register_fn(
        "shell",
        |cmd: &str| -> Result<Map, Box<EvalAltResult>> {
            run_blocking(Cmd::Shell(cmd.to_string()), &ShellOpts::default())
        },
    );
    // shell(argv_array)
    engine.register_fn(
        "shell",
        |argv: Array| -> Result<Map, Box<EvalAltResult>> {
            run_blocking(Cmd::Argv(coerce_argv(&argv)?), &ShellOpts::default())
        },
    );
    // shell(cmd_string, opts)
    engine.register_fn(
        "shell",
        |cmd: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            run_blocking(Cmd::Shell(cmd.to_string()), &ShellOpts::from_map(&opts)?)
        },
    );
    // shell(argv_array, opts)
    engine.register_fn(
        "shell",
        |argv: Array, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            run_blocking(Cmd::Argv(coerce_argv(&argv)?), &ShellOpts::from_map(&opts)?)
        },
    );

    // ---- streaming shell_stream() forms ------------------------------

    // shell_stream(cmd_string, callback)
    engine.register_fn(
        "shell_stream",
        |ctx: NativeCallContext, cmd: &str, callback: FnPtr|
         -> Result<i64, Box<EvalAltResult>> {
            run_streaming(&ctx, Cmd::Shell(cmd.to_string()), &callback, &ShellOpts::default())
        },
    );
    // shell_stream(argv_array, callback)
    engine.register_fn(
        "shell_stream",
        |ctx: NativeCallContext, argv: Array, callback: FnPtr|
         -> Result<i64, Box<EvalAltResult>> {
            run_streaming(&ctx, Cmd::Argv(coerce_argv(&argv)?), &callback, &ShellOpts::default())
        },
    );
    // shell_stream(cmd_string, callback, opts)
    engine.register_fn(
        "shell_stream",
        |ctx: NativeCallContext, cmd: &str, callback: FnPtr, opts: Map|
         -> Result<i64, Box<EvalAltResult>> {
            run_streaming(
                &ctx,
                Cmd::Shell(cmd.to_string()),
                &callback,
                &ShellOpts::from_map(&opts)?,
            )
        },
    );
    // shell_stream(argv_array, callback, opts)
    engine.register_fn(
        "shell_stream",
        |ctx: NativeCallContext, argv: Array, callback: FnPtr, opts: Map|
         -> Result<i64, Box<EvalAltResult>> {
            run_streaming(
                &ctx,
                Cmd::Argv(coerce_argv(&argv)?),
                &callback,
                &ShellOpts::from_map(&opts)?,
            )
        },
    );
}

// ---------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------

enum Cmd {
    /// Run through the platform shell (`sh -c` / `cmd /C`).
    Shell(String),
    /// Direct argv — first element is the program, rest are arguments.
    Argv(Vec<String>),
}

#[derive(Default)]
struct ShellOpts {
    cwd: Option<String>,
    env: HashMap<String, String>,
    env_clear: bool,
    timeout: Option<Duration>,
    merge_stderr: bool,
}

impl ShellOpts {
    fn from_map(m: &Map) -> Result<Self, Box<EvalAltResult>> {
        let mut o = ShellOpts::default();
        for (k, v) in m.iter() {
            match k.as_str() {
                "cwd" => {
                    o.cwd = Some(v.clone().into_string().map_err(|_| {
                        err("shell opts: `cwd` must be a string")
                    })?);
                }
                "env" => {
                    let env_map: Map = v.clone().try_cast().ok_or_else(|| {
                        err("shell opts: `env` must be a map")
                    })?;
                    for (ek, ev) in env_map.iter() {
                        let val = ev.clone().into_string().map_err(|_| {
                            err(format!("shell opts: env value for `{ek}` must be a string"))
                        })?;
                        o.env.insert(ek.to_string(), val);
                    }
                }
                "env_clear" => {
                    o.env_clear = v.clone().as_bool().unwrap_or(false);
                }
                "timeout_ms" => {
                    let ms = v.clone().as_int().map_err(|_| {
                        err("shell opts: `timeout_ms` must be an integer")
                    })?;
                    if ms > 0 {
                        o.timeout = Some(Duration::from_millis(ms as u64));
                    }
                }
                "merge_stderr" => {
                    o.merge_stderr = v.clone().as_bool().unwrap_or(false);
                }
                other => {
                    return Err(err(format!(
                        "shell opts: unknown key `{other}` \
                         (allowed: cwd, env, env_clear, timeout_ms, merge_stderr)"
                    )));
                }
            }
        }
        Ok(o)
    }
}

fn coerce_argv(arr: &Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    if arr.is_empty() {
        return Err(err("shell: argv array is empty"));
    }
    let mut out = Vec::with_capacity(arr.len());
    for (i, d) in arr.iter().enumerate() {
        let s = d.clone().into_string().map_err(|_| {
            err(format!("shell: argv[{i}] must be a string"))
        })?;
        out.push(s);
    }
    Ok(out)
}

fn build_command(cmd: &Cmd, opts: &ShellOpts) -> Command {
    let mut command = match cmd {
        Cmd::Shell(s) => {
            #[cfg(unix)]
            {
                let mut c = Command::new("sh");
                c.arg("-c").arg(s);
                c
            }
            #[cfg(not(unix))]
            {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(s);
                c
            }
        }
        Cmd::Argv(argv) => {
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        }
    };
    if let Some(cwd) = &opts.cwd {
        command.current_dir(cwd);
    }
    if opts.env_clear {
        command.env_clear();
    }
    for (k, v) in &opts.env {
        command.env(k, v);
    }
    command
}

fn run_blocking(cmd: Cmd, opts: &ShellOpts) -> Result<Map, Box<EvalAltResult>> {
    let mut command = build_command(&cmd, opts);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(if opts.merge_stderr { Stdio::piped() } else { Stdio::piped() });

    let mut child = command.spawn().map_err(|e| {
        err(format!("shell: spawn failed: {e}"))
    })?;

    // Drain stdout and stderr concurrently to avoid pipe-buffer deadlock.
    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");
    let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>();
    let (tx_err, rx_err) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let _ = drain_pipe(stdout, tx_out);
    });
    thread::spawn(move || {
        let _ = drain_pipe(stderr, tx_err);
    });

    // Wait for exit with optional timeout.
    let child_id = child.id();
    let (tx_done, rx_done) = mpsc::channel();
    thread::spawn(move || {
        let status = child.wait();
        let _ = tx_done.send(status);
    });

    let status = match opts.timeout {
        Some(d) => match rx_done.recv_timeout(d) {
            Ok(s) => s,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                kill_pid(child_id);
                return Err(err(format!(
                    "shell: timed out after {} ms",
                    d.as_millis()
                )));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(err("shell: wait thread disconnected"));
            }
        },
        None => rx_done.recv().map_err(|_| {
            err("shell: wait thread disconnected")
        })?,
    }
    .map_err(|e| err(format!("shell: wait failed: {e}")))?;

    let stdout_bytes: Vec<u8> = rx_out.iter().flatten().collect();
    let stderr_bytes: Vec<u8> = rx_err.iter().flatten().collect();

    let mut m = Map::new();
    let stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).into_owned();
    if opts.merge_stderr {
        let combined = format!("{stdout_str}{stderr_str}");
        m.insert("stdout".into(), combined.into());
        m.insert("stderr".into(), "".to_string().into());
    } else {
        m.insert("stdout".into(), stdout_str.into());
        m.insert("stderr".into(), stderr_str.into());
    }
    let code = status.code().unwrap_or(-1) as i64;
    m.insert("exit_code".into(), code.into());
    m.insert("success".into(), (code == 0).into());
    Ok(m)
}

fn drain_pipe<R: std::io::Read + Send + 'static>(
    pipe: R,
    tx: mpsc::Sender<Vec<u8>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(pipe);
    let mut buf = [0u8; 4096];
    loop {
        let n = reader.get_mut().read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        if tx.send(buf[..n].to_vec()).is_err() {
            return Ok(());
        }
    }
}

fn run_streaming(
    ctx: &NativeCallContext,
    cmd: Cmd,
    callback: &FnPtr,
    opts: &ShellOpts,
) -> Result<i64, Box<EvalAltResult>> {
    let mut command = build_command(&cmd, opts);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| {
        err(format!("shell_stream: spawn failed: {e}"))
    })?;
    let started = Instant::now();
    let child_id = child.id();

    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");

    // Both stdout and stderr drain into the same channel — the
    // streaming form always merges, matching what a user staring at
    // a pane would expect (the order of stdout/stderr writes is
    // already serialised by the OS).
    let (tx, rx) = mpsc::channel::<String>();
    let tx_out = tx.clone();
    thread::spawn(move || forward_lines(stdout, tx_out));
    thread::spawn(move || forward_lines(stderr, tx));

    let (tx_done, rx_done) = mpsc::channel();
    thread::spawn(move || {
        let status = child.wait();
        let _ = tx_done.send(status);
    });

    // Pull lines as they arrive and fire the callback on each.
    // Apply the deadline as an upper bound on the whole call, not
    // per-line (a slow command that writes nothing for a while still
    // shouldn't hang us forever).
    let deadline = opts.timeout.map(|d| started + d);
    loop {
        let remaining = match deadline {
            Some(d) => d.checked_duration_since(Instant::now()),
            None => None,
        };
        let recv = if let Some(d) = deadline {
            if remaining.is_none() {
                kill_pid(child_id);
                return Err(err(format!(
                    "shell_stream: timed out after {} ms",
                    d.duration_since(started).as_millis()
                )));
            }
            rx.recv_timeout(remaining.unwrap())
        } else {
            // No deadline — poll with a short timeout so we can also
            // notice child-exit between line bursts.
            match rx.recv_timeout(Duration::from_millis(100)) {
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check if the process has finished and the
                    // forwarder threads have closed their sender ends.
                    if let Ok(_status) = rx_done.try_recv() {
                        // Drain any straggling lines from the channel.
                        drain_remaining(&rx, ctx, callback)?;
                        return Ok(_status
                            .map(|s| s.code().unwrap_or(-1) as i64)
                            .unwrap_or(-1));
                    }
                    continue;
                }
                other => other,
            }
        };

        match recv {
            Ok(line) => {
                fire_callback(ctx, callback, &line)?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Deadline hit.
                kill_pid(child_id);
                return Err(err("shell_stream: timed out"));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Both forwarder threads dropped the sender — the
                // pipes are EOF. Wait for the process to finish.
                let status = rx_done.recv().map_err(|_| {
                    err("shell_stream: wait thread disconnected")
                })?;
                let code = status
                    .map(|s| s.code().unwrap_or(-1) as i64)
                    .unwrap_or(-1);
                return Ok(code);
            }
        }
    }
}

fn drain_remaining(
    rx: &mpsc::Receiver<String>,
    ctx: &NativeCallContext,
    callback: &FnPtr,
) -> Result<(), Box<EvalAltResult>> {
    while let Ok(line) = rx.try_recv() {
        fire_callback(ctx, callback, &line)?;
    }
    Ok(())
}

fn fire_callback(
    ctx: &NativeCallContext,
    callback: &FnPtr,
    line: &str,
) -> Result<(), Box<EvalAltResult>> {
    let line_dyn: Dynamic = line.to_string().into();
    // call_within_context runs the FnPtr against the current engine +
    // scope, which is what the script-side callback expects (captured
    // variables work, fn definitions resolve, etc.).
    let _: Dynamic = callback.call_within_context(ctx, (line_dyn,))?;
    Ok(())
}

fn forward_lines<R: std::io::Read>(pipe: R, tx: mpsc::Sender<String>) {
    let reader = BufReader::new(pipe);
    for line in reader.lines().map_while(Result::ok) {
        if tx.send(line).is_err() {
            return;
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
    }
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        e
    }

    #[test]
    fn shell_blocking_captures_stdout_and_exit_code() {
        let r: Map = engine()
            .eval(r#"shell("echo hello")"#)
            .unwrap();
        assert_eq!(
            r.get("stdout").unwrap().clone().into_string().unwrap(),
            "hello\n"
        );
        assert_eq!(r.get("exit_code").unwrap().as_int().unwrap(), 0);
        assert_eq!(r.get("success").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn shell_blocking_nonzero_exit_reports_failure() {
        let r: Map = engine()
            .eval(r#"shell("sh -c 'exit 7'")"#)
            .unwrap();
        assert_eq!(r.get("exit_code").unwrap().as_int().unwrap(), 7);
        assert_eq!(r.get("success").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn shell_argv_form_skips_shell_layer() {
        let r: Map = engine()
            .eval(r#"shell(["echo", "$HOME"])"#)
            .unwrap();
        // No shell expansion — argv form prints the literal "$HOME".
        assert_eq!(
            r.get("stdout").unwrap().clone().into_string().unwrap(),
            "$HOME\n"
        );
    }

    #[test]
    fn shell_opts_cwd_takes_effect() {
        let r: Map = engine()
            .eval(r#"shell("pwd", #{ cwd: "/tmp" })"#)
            .unwrap();
        let out = r.get("stdout").unwrap().clone().into_string().unwrap();
        // /tmp may resolve to /private/tmp on macOS; both are fine.
        assert!(out.trim().ends_with("/tmp"), "got {out:?}");
    }

    #[test]
    fn shell_opts_env_layered_on_parent() {
        let r: Map = engine()
            .eval(r#"shell("echo $RECON_TEST_FOO", #{ env: #{ RECON_TEST_FOO: "bar" } })"#)
            .unwrap();
        assert_eq!(
            r.get("stdout").unwrap().clone().into_string().unwrap(),
            "bar\n"
        );
    }

    #[test]
    fn shell_opts_timeout_kills_long_running() {
        let result: Result<Map, _> = engine()
            .eval(r#"shell("sleep 5", #{ timeout_ms: 100 })"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("timed out"), "got {msg}");
    }

    #[test]
    fn shell_stream_invokes_callback_per_line() {
        let mut e = engine();
        // Use a counter map so we can collect lines from the callback
        // (Rhai closures captures shared state via push_constant /
        // module bindings — easiest path is a global).
        let script = r#"
            let lines = [];
            shell_stream("printf 'a\nb\nc\n'", |line| lines.push(line));
            lines
        "#;
        let r: Array = e.eval(script).unwrap();
        let collected: Vec<String> = r
            .into_iter()
            .map(|d| d.into_string().unwrap())
            .collect();
        assert_eq!(collected, vec!["a", "b", "c"]);
    }

    #[test]
    fn shell_stream_returns_exit_code() {
        let mut e = engine();
        let code: i64 = e
            .eval(r#"shell_stream("sh -c 'echo hi; exit 3'", |line| ())"#)
            .unwrap();
        assert_eq!(code, 3);
    }

    #[test]
    fn shell_argv_empty_array_errors() {
        let mut e = engine();
        let result: Result<Map, _> = e.eval(r#"shell([])"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("argv array is empty"), "got {msg}");
    }
}
