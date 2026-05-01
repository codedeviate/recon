//! `agentBrowser::*` static module — wraps the `agent-browser` CLI.
//!
//! Registered unconditionally. When the binary isn't available, the
//! constants `agentBrowser::available` (false) and `agentBrowser::version`
//! ("") are still readable; every function call surfaces a Rhai error
//! asking the user to install agent-browser.
//!
//! Execution delegates to `crate::agent_browser::run_cmd`.

use crate::agent_browser;
use crate::script::bindings::helpers::json_to_dynamic;
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();
    let state = agent_browser::state_snapshot();

    module.set_var("available", state.available);
    module.set_var("version", state.version);

    // Simple wrappers that return stdout as a String.
    register_simple(&mut module, "open", |a: &[String]| {
        vec!["open".to_string(), a[0].clone()]
    }, 1);
    register_simple(&mut module, "close", |_| vec!["close".to_string()], 0);
    register_simple(&mut module, "close_all", |_| {
        vec!["close".into(), "--all".into()]
    }, 0);

    for cmd in [
        "click", "dblclick", "hover", "focus", "check", "uncheck",
        "scrollintoview", "back", "forward", "reload",
    ] {
        let name = to_rhai_name(cmd);
        if matches!(cmd, "back" | "forward" | "reload") {
            // no-arg navigation verbs
            let c = cmd.to_string();
            register_simple(&mut module, &name, move |_| vec![c.clone()], 0);
        } else {
            let c = cmd.to_string();
            register_simple(&mut module, &name, move |a| {
                vec![c.clone(), a[0].clone()]
            }, 1);
        }
    }

    // Two-arg: sel + text/value
    for cmd in ["fill"] {
        let c = cmd.to_string();
        register_simple(&mut module, cmd, move |a| {
            vec![c.clone(), a[0].clone(), a[1].clone()]
        }, 2);
    }
    // `type <sel> <text>` — Rhai-side name `type_text`
    register_simple(&mut module, "type_text", |a| {
        vec!["type".to_string(), a[0].clone(), a[1].clone()]
    }, 2);

    // Single-arg: press, wait, pdf
    for cmd in ["press", "wait", "pdf"] {
        let c = cmd.to_string();
        register_simple(&mut module, cmd, move |a| {
            vec![c.clone(), a[0].clone()]
        }, 1);
    }

    // scroll(dir) + scroll(dir, px)
    register_simple(&mut module, "scroll", |a| {
        vec!["scroll".to_string(), a[0].clone()]
    }, 1);
    let _ = module.set_native_fn(
        "scroll",
        |dir: &str, px: i64| -> Result<String, Box<EvalAltResult>> {
            run_string(&["scroll", dir, &px.to_string()])
        },
    );

    // screenshot() + screenshot(path)
    register_simple(&mut module, "screenshot", |_| {
        vec!["screenshot".to_string()]
    }, 0);
    let _ = module.set_native_fn(
        "screenshot",
        |path: &str| -> Result<String, Box<EvalAltResult>> {
            run_string(&["screenshot", path])
        },
    );

    // keyboard type <text> / keyboard inserttext <text>
    let _ = module.set_native_fn(
        "keyboard_type",
        |text: &str| -> Result<String, Box<EvalAltResult>> {
            run_string(&["keyboard", "type", text])
        },
    );
    let _ = module.set_native_fn(
        "keyboard_insert",
        |text: &str| -> Result<String, Box<EvalAltResult>> {
            run_string(&["keyboard", "inserttext", text])
        },
    );

    // is_visible / is_enabled / is_checked
    // JSON envelope: { success: true, data: { visible: true, ... } }
    // After run_json unwraps `data`, we look for the field named after
    // the predicate (visible / enabled / checked).
    for what in ["visible", "enabled", "checked"] {
        let name = format!("is_{what}");
        let w = what.to_string();
        let _ = module.set_native_fn(
            name.as_str(),
            move |sel: &str| -> Result<bool, Box<EvalAltResult>> {
                let data = run_json(&["is", &w, sel])?;
                if let Some(map) = data.clone().try_cast::<rhai::Map>() {
                    if let Some(v) = map.get(w.as_str()) {
                        if let Ok(b) = v.as_bool() {
                            return Ok(b);
                        }
                    }
                }
                Err(err(format!(
                    "agent-browser: is {w} returned unexpected payload"
                )))
            },
        );
    }

    // snapshot() / snapshot(true) → parsed JSON
    let _ = module.set_native_fn(
        "snapshot",
        || -> Result<Dynamic, Box<EvalAltResult>> { run_json(&["snapshot"]) },
    );
    let _ = module.set_native_fn(
        "snapshot",
        |interactive: bool| -> Result<Dynamic, Box<EvalAltResult>> {
            if interactive {
                run_json(&["snapshot", "-i"])
            } else {
                run_json(&["snapshot"])
            }
        },
    );

    // eval(js) — JSON-parse the result best-effort.
    let _ = module.set_native_fn(
        "eval",
        |js: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_json(&["eval", js])
        },
    );

    // get(what) / get(what, sel) — JSON-parse.
    let _ = module.set_native_fn(
        "get",
        |what: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_json(&["get", what])
        },
    );
    let _ = module.set_native_fn(
        "get",
        |what: &str, sel: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_json(&["get", what, sel])
        },
    );

    // find(locator, value, action) / find(locator, value, action, text)
    let _ = module.set_native_fn(
        "find",
        |loc: &str, val: &str, action: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_json(&["find", loc, val, action])
        },
    );
    let _ = module.set_native_fn(
        "find",
        |loc: &str, val: &str, action: &str, text: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_json(&["find", loc, val, action, text])
        },
    );

    // cmd(args_array) — arbitrary CLI invocation, raw stdout.
    let _ = module.set_native_fn(
        "cmd",
        |args: Array| -> Result<String, Box<EvalAltResult>> {
            let owned: Vec<String> = args
                .into_iter()
                .map(|v| {
                    if v.is_string() {
                        v.into_string().unwrap_or_default()
                    } else {
                        v.to_string()
                    }
                })
                .collect();
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            run_string(&refs)
        },
    );

    engine.register_static_module("agentBrowser", module.into());
}

fn to_rhai_name(cli: &str) -> String {
    cli.replace('-', "_")
}

/// Shared closure registration for single-shape wrappers that return a
/// String. The `build_args` closure turns the incoming Rhai arg list
/// into the real CLI argv.
fn register_simple<F>(module: &mut Module, name: &str, build: F, arity: usize)
where
    F: Fn(&[String]) -> Vec<String> + Send + Sync + Clone + 'static,
{
    match arity {
        0 => {
            let b = build.clone();
            let _ = module.set_native_fn(
                name,
                move || -> Result<String, Box<EvalAltResult>> {
                    let argv = b(&[]);
                    run_string_owned(&argv)
                },
            );
        }
        1 => {
            let b = build.clone();
            let _ = module.set_native_fn(
                name,
                move |a1: &str| -> Result<String, Box<EvalAltResult>> {
                    let argv = b(&[a1.to_string()]);
                    run_string_owned(&argv)
                },
            );
        }
        2 => {
            let b = build.clone();
            let _ = module.set_native_fn(
                name,
                move |a1: &str, a2: &str| -> Result<String, Box<EvalAltResult>> {
                    let argv = b(&[a1.to_string(), a2.to_string()]);
                    run_string_owned(&argv)
                },
            );
        }
        _ => unreachable!("register_simple only handles arities 0..=2"),
    }
}

fn run_string(args: &[&str]) -> Result<String, Box<EvalAltResult>> {
    agent_browser::run_cmd(args, false).map_err(|e| err(e.to_string()))
}

fn run_string_owned(args: &[String]) -> Result<String, Box<EvalAltResult>> {
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_string(&refs)
}

fn run_json(args: &[&str]) -> Result<Dynamic, Box<EvalAltResult>> {
    let raw = agent_browser::run_cmd(args, true).map_err(|e| err(e.to_string()))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Dynamic::UNIT);
    }
    let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Ok(Dynamic::from(raw)),
    };

    // agent-browser JSON envelopes are `{ success: bool, data: <payload>,
    // error: string | null }`. Unwrap so scripts see the payload directly.
    if let Some(obj) = parsed.as_object() {
        if let Some(success) = obj.get("success").and_then(|v| v.as_bool()) {
            if !success {
                let msg = obj
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent-browser: command failed")
                    .to_string();
                return Err(err(format!("agent-browser: {msg}")));
            }
            if let Some(data) = obj.get("data") {
                return Ok(json_to_dynamic(data.clone()));
            }
        }
    }
    // Not an envelope — return the parsed value as-is.
    Ok(json_to_dynamic(parsed))
}

// ── Option translation ───────────────────────────────────────────────────────

/// Bool flags: emit `--<flag>` when value is true, omit otherwise.
const BOOL_OPTS: &[&str] = &[
    "ignore_https_errors",
    "allow_file_access",
    "headed",
    "auto_connect",
    "annotate",
    "no_auto_dialog",
    "content_boundaries",
    "confirm_interactive",
    "verbose",
    "quiet",
    "debug",
    "json",
];

/// String flags: emit `--<flag> <value>`.
const STRING_OPTS: &[&str] = &[
    "session",
    "session_name",
    "executable_path",
    "user_agent",
    "proxy",
    "proxy_bypass",
    "state",
    "profile",
    "provider",
    "device",
    "color_scheme",
    "engine",
    "model",
    "config",
    "screenshot_dir",
    "screenshot_format",
    "download_path",
    "allowed_domains",
    "action_policy",
    "confirm_actions",
];

/// Int flags: emit `--<flag> <int>`.
const INT_OPTS: &[&str] = &["cdp", "screenshot_quality", "max_output"];

/// Repeatable flags: accept str or array-of-str. Emit `--<flag> <v>` per
/// entry. `browser_args` is the Rhai-side rename of agent-browser's
/// `--args` (the literal name `args` is too generic).
const REPEATABLE_OPTS: &[(&str, &str)] = &[
    ("extension", "extension"),
    ("browser_args", "args"),
];

/// Translate snake_case to kebab-case for emitting CLI flag names.
fn key_to_flag(key: &str) -> String {
    format!("--{}", key.replace('_', "-"))
}

/// Translate a Rhai opts map into an agent-browser argv prefix.
/// Returns Err with a helpful message on unknown keys, type mismatches,
/// or `headers` JSON-serialization failure.
pub(crate) fn opts_to_argv(opts: &rhai::Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut argv: Vec<String> = Vec::new();

    for (k, v) in opts.iter() {
        let key = k.as_str();

        if BOOL_OPTS.contains(&key) {
            let b = v.as_bool().map_err(|got| {
                err(format!(
                    "agentBrowser: option '{key}' expects bool, got {got}"
                ))
            })?;
            if b {
                argv.push(key_to_flag(key));
            }
            continue;
        }
        if STRING_OPTS.contains(&key) {
            let s = v.clone().into_string().map_err(|got| {
                err(format!(
                    "agentBrowser: option '{key}' expects string, got {got}"
                ))
            })?;
            argv.push(key_to_flag(key));
            argv.push(s);
            continue;
        }
        if INT_OPTS.contains(&key) {
            let n = v.as_int().map_err(|got| {
                err(format!(
                    "agentBrowser: option '{key}' expects int, got {got}"
                ))
            })?;
            argv.push(key_to_flag(key));
            argv.push(n.to_string());
            continue;
        }
        if let Some((_, cli_name)) = REPEATABLE_OPTS.iter().find(|(rhai, _)| *rhai == key) {
            let entries = repeatable_to_strings(key, v.clone())?;
            for entry in entries {
                argv.push(format!("--{cli_name}"));
                argv.push(entry);
            }
            continue;
        }
        if key == "headers" {
            argv.push("--headers".to_string());
            argv.push(headers_to_json(v.clone())?);
            continue;
        }

        return Err(err(format!(
            "agentBrowser: unknown option '{key}' (valid: {})",
            valid_keys_csv()
        )));
    }
    Ok(argv)
}

fn repeatable_to_strings(
    key: &str,
    v: rhai::Dynamic,
) -> Result<Vec<String>, Box<EvalAltResult>> {
    if let Ok(s) = v.clone().into_string() {
        return Ok(vec![s]);
    }
    if let Some(arr) = v.try_cast::<rhai::Array>() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, item) in arr.into_iter().enumerate() {
            let s = item.into_string().map_err(|got| {
                err(format!(
                    "agentBrowser: option '{key}'[{i}] expects string, got {got}"
                ))
            })?;
            out.push(s);
        }
        return Ok(out);
    }
    Err(err(format!(
        "agentBrowser: option '{key}' expects string or array of strings"
    )))
}

fn headers_to_json(v: rhai::Dynamic) -> Result<String, Box<EvalAltResult>> {
    if let Ok(s) = v.clone().into_string() {
        return Ok(s);
    }
    if let Some(map) = v.try_cast::<rhai::Map>() {
        let mut json_map = serde_json::Map::new();
        for (k, vv) in map {
            let key = k.to_string();
            let value = rhai_to_json(vv)?;
            json_map.insert(key, value);
        }
        return serde_json::to_string(&json_map)
            .map_err(|e| err(format!("agentBrowser: headers JSON: {e}")));
    }
    Err(err(
        "agentBrowser: option 'headers' expects string or map".to_string(),
    ))
}

fn rhai_to_json(v: rhai::Dynamic) -> Result<serde_json::Value, Box<EvalAltResult>> {
    if v.is_unit() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(b) = v.as_bool() {
        return Ok(serde_json::Value::Bool(b));
    }
    if let Ok(n) = v.as_int() {
        return Ok(serde_json::Value::Number(n.into()));
    }
    if let Ok(s) = v.clone().into_string() {
        return Ok(serde_json::Value::String(s));
    }
    Err(err(format!(
        "agentBrowser: headers value of unsupported type: {}",
        v.type_name()
    )))
}

fn valid_keys_csv() -> String {
    let mut keys: Vec<&str> = Vec::new();
    keys.extend(BOOL_OPTS);
    keys.extend(STRING_OPTS);
    keys.extend(INT_OPTS);
    keys.extend(REPEATABLE_OPTS.iter().map(|(rhai, _)| *rhai));
    keys.push("headers");
    keys.sort();
    keys.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::defaults::ScriptDefaults;

    fn engine_with_ab() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        // helpers for assert etc.
        crate::script::bindings::helpers::register(&mut e);
        e
    }

    #[test]
    fn module_has_available_and_version_constants() {
        let e = engine_with_ab();
        // These are always present.
        let avail: bool = e.eval("agentBrowser::available").expect("eval");
        let version: String = e.eval("agentBrowser::version").expect("eval");
        // Binary may or may not be installed on the test host — just
        // assert the types are what scripts expect.
        let _ = avail;
        let _ = version;
    }

    #[test]
    fn version_is_empty_when_unavailable_bool_invariant() {
        let e = engine_with_ab();
        let script = r#"
if agentBrowser::available {
    agentBrowser::version.len() > 0
} else {
    agentBrowser::version == ""
}
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn guard_pattern_compiles_and_runs() {
        // The documented guard pattern should evaluate cleanly whether
        // agent-browser is installed or not.
        let e = engine_with_ab();
        let script = r#"
if !agentBrowser::available { return 2; }
return 0;
"#;
        let code: i64 = e.eval(script).expect("eval");
        assert!(code == 0 || code == 2);
    }

    #[test]
    fn to_rhai_name_replaces_hyphens() {
        assert_eq!(to_rhai_name("scrollintoview"), "scrollintoview");
        assert_eq!(to_rhai_name("some-name"), "some_name");
    }

    #[allow(dead_code)]
    fn unused_defaults_ref(_d: &ScriptDefaults) {}

    // ── opts_to_argv tests ───────────────────────────────────────────────

    #[test]
    fn opts_to_argv_bool_true_emits_flag() {
        let mut m = rhai::Map::new();
        m.insert("ignore_https_errors".into(), true.into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec!["--ignore-https-errors".to_string()]
        );
    }

    #[test]
    fn opts_to_argv_bool_false_omits_flag() {
        let mut m = rhai::Map::new();
        m.insert("ignore_https_errors".into(), false.into());
        assert!(opts_to_argv(&m).unwrap().is_empty());
    }

    #[test]
    fn opts_to_argv_string_flag() {
        let mut m = rhai::Map::new();
        m.insert("user_agent".into(), "Recon/0.75".into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec!["--user-agent".to_string(), "Recon/0.75".to_string()]
        );
    }

    #[test]
    fn opts_to_argv_int_flag() {
        let mut m = rhai::Map::new();
        m.insert("cdp".into(), (9222_i64).into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec!["--cdp".to_string(), "9222".to_string()]
        );
    }

    #[test]
    fn opts_to_argv_repeatable_string_single() {
        let mut m = rhai::Map::new();
        m.insert("extension".into(), "/path/to/ext".into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec!["--extension".to_string(), "/path/to/ext".to_string()]
        );
    }

    #[test]
    fn opts_to_argv_repeatable_array() {
        let mut m = rhai::Map::new();
        let arr: rhai::Array = vec!["a".into(), "b".into()];
        m.insert("extension".into(), arr.into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec![
                "--extension".to_string(),
                "a".to_string(),
                "--extension".to_string(),
                "b".to_string(),
            ]
        );
    }

    #[test]
    fn opts_to_argv_browser_args_renames_to_args() {
        let mut m = rhai::Map::new();
        m.insert("browser_args".into(), "--no-sandbox".into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec!["--args".to_string(), "--no-sandbox".to_string()]
        );
    }

    #[test]
    fn opts_to_argv_headers_string_passthrough() {
        let mut m = rhai::Map::new();
        m.insert("headers".into(), r#"{"X-Foo":"bar"}"#.into());
        assert_eq!(
            opts_to_argv(&m).unwrap(),
            vec![
                "--headers".to_string(),
                r#"{"X-Foo":"bar"}"#.to_string()
            ]
        );
    }

    #[test]
    fn opts_to_argv_headers_map_serialized() {
        let mut hdrs = rhai::Map::new();
        hdrs.insert("X-Foo".into(), "bar".into());
        let mut m = rhai::Map::new();
        m.insert("headers".into(), hdrs.into());
        let argv = opts_to_argv(&m).unwrap();
        assert_eq!(argv[0], "--headers");
        let parsed: serde_json::Value = serde_json::from_str(&argv[1]).unwrap();
        assert_eq!(parsed["X-Foo"], "bar");
    }

    #[test]
    fn opts_to_argv_unknown_key_errors_with_listing() {
        let mut m = rhai::Map::new();
        m.insert("does_not_exist".into(), true.into());
        let e = opts_to_argv(&m).unwrap_err().to_string();
        assert!(e.contains("does_not_exist"));
        assert!(e.contains("ignore_https_errors")); // listed
    }

    #[test]
    fn opts_to_argv_type_mismatch_errors() {
        let mut m = rhai::Map::new();
        m.insert("ignore_https_errors".into(), "true".into()); // wrong: string
        let e = opts_to_argv(&m).unwrap_err().to_string();
        assert!(e.contains("ignore_https_errors"));
        assert!(e.contains("bool"));
    }
}
