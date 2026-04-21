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
}
