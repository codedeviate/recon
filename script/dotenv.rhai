// dotenv.rhai — load .env files and read process env from a script.
//
// Usage: recon --script dotenv
//
// Demonstrates two new bindings (0.76.0):
//
//   load_dotenv(path)                — parse a .env file, set each KEY=VALUE
//                                      in the process env (overrides existing).
//   load_dotenv(path, override)      — same, but explicit. Pass `false` to
//                                      leave pre-existing shell-env vars alone.
//   env_all()                        — snapshot the entire process env as a Map.
//
// Camel-case aliases `loadDotEnv` and `envAll` are also registered for
// callers who prefer that style.
//
// Pattern: a directory holds multiple scripts that share a common .env
// plus per-script overrides. Load common first, specific second — the
// later load wins.

let common = "/tmp/recon-dotenv-demo.env";
let specific = "/tmp/recon-dotenv-demo.env.greet";
file_write_all(common, "API_HOST=api.example.com\nLOG_LEVEL=info\n");
file_write_all(specific, "LOG_LEVEL=debug\nGREETING=hello\n");

// Load order: common → script-specific. The second load overrides
// LOG_LEVEL, leaving API_HOST untouched and adding GREETING.
let n_common = load_dotenv(common);
let n_specific = load_dotenv(specific);
print(`loaded ${n_common} from common, ${n_specific} from script-specific`);

print(`API_HOST=${env("API_HOST")}`);
print(`LOG_LEVEL=${env("LOG_LEVEL")}`);   // debug — specific wins
print(`GREETING=${env("GREETING")}`);

// Non-override variant: leave any pre-existing env vars in place. If
// LOG_LEVEL was already set by the user's shell, this call would not
// overwrite it. Useful when you want shell env to take priority over
// .env defaults.
load_dotenv(common, false);

// env_all() — handy for logging or filtering the whole environment.
let everything = env_all();
print(`env_all() returned ${everything.len()} variables`);

// camelCase aliases work too.
loadDotEnv(specific);
let m = envAll();
print(`envAll has GREETING = ${m["GREETING"]}`);

// Tidy up.
file_delete(common);
file_delete(specific);
