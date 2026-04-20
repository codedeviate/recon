//! `redis(url)` / `redis(url, command)` script binding.
//!
//! `redis(url)` sends PING. `redis(url, "GET foo")` or
//! `redis(url, "SET key \"hello world\"")` sends an arbitrary RESP2
//! command — tokens split shell-style (whitespace + `"…"` + `'…'` +
//! `\`-escapes), matching the CLI's `redis:// -d 'CMD args'` behaviour.
//!
//! Returns `#{ host, port, connect_ms, auth_reply, command, reply,
//! command_ms }`. AUTH failure exits 67; connect refused exits 7;
//! timeout exits 28.

use crate::redis_probe;
use crate::script::convert::{anyhow_to_rhai, err, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn(
            "redis",
            move |url: &str| -> Result<Map, Box<EvalAltResult>> { do_redis(url, None, d.connect_timeout) },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "redis",
            move |url: &str, command: &str| -> Result<Map, Box<EvalAltResult>> {
                let toks = redis_probe::shell_split(command)
                    .ok_or_else(|| err("redis: unbalanced quotes in command"))?;
                if toks.is_empty() {
                    return Err(err("redis: command must not be empty"));
                }
                do_redis(url, Some(toks), d.connect_timeout)
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "redis",
            move |url: &str, command: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let toks = redis_probe::shell_split(command)
                    .ok_or_else(|| err("redis: unbalanced quotes in command"))?;
                if toks.is_empty() {
                    return Err(err("redis: command must not be empty"));
                }
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_redis(url, Some(toks), timeout)
            },
        );
    }
}

fn do_redis(
    url: &str,
    command_args: Option<Vec<String>>,
    timeout_secs: u64,
) -> Result<Map, Box<EvalAltResult>> {
    let r = redis_probe::probe(url, command_args, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("host".into(), r.host.into());
    m.insert("port".into(), (r.port as i64).into());
    m.insert("connect_ms".into(), r.connect_ms.into());
    if let Some(a) = r.auth_reply {
        m.insert("auth_reply".into(), a.into());
    }
    m.insert("command".into(), r.command_label.into());
    m.insert("reply".into(), r.reply.into());
    m.insert("command_ms".into(), r.command_ms.into());
    Ok(m)
}
