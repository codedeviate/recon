// dotenv.rhai — load .env files alongside the script (no tempfile dance).
//
// Usage: recon --script dotenv [OVERLAY_NAME]
//
// Demonstrates the directory-overlay pattern. Drop this script, a shared
// `.env`, and any number of `.env.<name>` overlays in the same directory:
//
//   /path/to/scripts/
//   ├── dotenv.rhai
//   ├── .env                   ← shared across every script in the dir
//   ├── .env.dotenv            ← per-script overlay (matches args[0])
//   └── .env.staging           ← optional named overlay (passed as arg)
//
// `script_dir` and `script_name` (added in 0.76.1) hold the resolved
// directory containing the script and its file stem (basename minus
// extension). Combine them with .env names to load files siblings to
// the script regardless of where `recon` is invoked from.
//
// `load_dotenv(path)` overrides existing values by default, so the overlay
// loaded second wins for any key it redefines. Use `load_dotenv(path, false)`
// to leave existing env (e.g. shell exports) untouched.

print(`script_dir  = ${script_dir}`);
print(`script_path = ${script_path}`);
print(`script_name = ${script_name}`);

// 1. Shared .env, sibling to the script. Quietly skip when missing so the
//    demo is runnable even from a fresh checkout that has no .env yet.
let common = script_dir + "/.env";
try {
    let n = load_dotenv(common);
    print(`loaded ${n} vars from ${common}`);
} catch(e) {
    print(`no shared .env (${common})`);
}

// 2. Per-script overlay. By default we look for `.env.<script_name>`;
//    pass a positional arg to override (e.g. `recon --script dotenv
//    staging` looks for `.env.staging` instead).
let overlay_name = if args.len() > 1 { args[1] } else { script_name };
let overlay = script_dir + "/.env." + overlay_name;
try {
    let n = load_dotenv(overlay);
    print(`loaded ${n} vars from ${overlay}`);
} catch(e) {
    print(`no overlay (${overlay})`);
}

// 3. Read whatever ended up set. `env(NAME, DEFAULT)` keeps the script
//    runnable when the .env files don't exist yet.
print(`API_HOST   = ${env("API_HOST", "(unset)")}`);
print(`LOG_LEVEL  = ${env("LOG_LEVEL", "(unset)")}`);
print(`GREETING   = ${env("GREETING", "(unset)")}`);

// 4. env_all() snapshot — handy for logging or filtering the whole env.
let everything = env_all();
print(`env_all() returned ${everything.len()} variables`);
