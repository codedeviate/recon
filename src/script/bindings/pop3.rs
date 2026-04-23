//! `pop3(url)` / `pop3(url, opts)` — probe or retrieve a POP3 message.
//!
//! opts: #{ user, pass, stls (bool), insecure (bool), timeout_ms (int) }.
//! Returns: #{ host, port, tls, banner, capabilities: Array,
//! message_count, total_bytes, message, connect_ms }.

use crate::pop3_probe::{self, Pop3Args};
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_str, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("pop3", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_pop3(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "pop3",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_pop3(url, &d, Some(&opts))
            },
        );
    }
}

fn do_pop3(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    let user = opts.and_then(|o| opts_get_str(o, "user"));
    let pass = opts.and_then(|o| opts_get_str(o, "pass"));
    let stls = opts.and_then(|o| opts_get_bool(o, "stls")).unwrap_or(false);
    let insecure = opts
        .and_then(|o| opts_get_bool(o, "insecure"))
        .unwrap_or(defaults.insecure);
    let timeout = opts
        .and_then(|o| opts_get_u64(o, "timeout_ms"))
        .map(|ms| ms / 1000)
        .unwrap_or(defaults.connect_timeout)
        .max(1);

    let pargs = Pop3Args {
        user: user.as_deref(),
        pass: pass.as_deref(),
        stls,
        insecure,
        timeout_secs: timeout,
    };
    let r = pop3_probe::probe(url, &pargs).map_err(anyhow_to_rhai)?;

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("tls".into(), r.tls.into());
    out.insert("banner".into(), r.banner.into());
    out.insert("connect_ms".into(), r.connect_ms.into());
    let caps: Array = r.capabilities.into_iter().map(Dynamic::from).collect();
    out.insert("capabilities".into(), caps.into());
    out.insert(
        "message_count".into(),
        match r.message_count { Some(n) => (n as i64).into(), None => Dynamic::UNIT },
    );
    out.insert(
        "total_bytes".into(),
        match r.total_bytes { Some(n) => (n as i64).into(), None => Dynamic::UNIT },
    );
    out.insert(
        "message".into(),
        match r.message {
            Some(b) => String::from_utf8_lossy(&b).into_owned().into(),
            None => Dynamic::UNIT,
        },
    );
    Ok(out)
}
