// shell.rhai — run subprocesses from a script.
// Usage: recon --script shell.rhai

// Blocking form — returns a Map with stdout / stderr / exit_code / success.
let r = shell("uname -s");
print(`OS:        ${trim(r.stdout)}`);
print(`exit code: ${r.exit_code}, success: ${r.success}`);

// argv form skips the shell layer entirely — no $VAR expansion, no
// quoting surprises. Use it when the command name comes from untrusted
// input.
let r = shell(["echo", "literal $HOME"]);
print(`argv form: ${trim(r.stdout)}`);

// Options map: cwd, env (layered on parent), env_clear, timeout_ms,
// merge_stderr.
let r = shell("echo $GREETING from $(pwd | xargs basename)", #{
    cwd: "/tmp",
    env: #{ GREETING: "hi" }
});
print(`opts:      ${trim(r.stdout)}`);

// Streaming form — the callback fires once per line as the child
// writes it. Good for live progress UIs and the upcoming TUI panes.
print("--- streaming ---");
let n = 0;
let exit = shell_stream("for i in 1 2 3; do echo step $i; done", |line| {
    n += 1;
    print(`  [#${n}] ${line}`);
});
print(`streamed ${n} lines, exit = ${exit}`);

// timeout_ms kills runaway children and raises a Rhai error.
try {
    shell("sleep 10", #{ timeout_ms: 100 });
} catch (e) {
    print(`timeout caught: ${e}`);
}

// Exit cleanly — recon maps a trailing integer to the process exit code.
()
