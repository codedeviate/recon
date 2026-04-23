//! `gopher(url)` / `gopher(url, opts)` — RFC 1436 text probe.
//!
//! opts: #{ insecure (bool), timeout_ms (int) }.
//! Returns: #{ host, port, tls, selector, item_type, connect_ms,
//! content (String, lossy UTF-8), bytes (Blob) }.

use crate::gopher_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("gopher", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_gopher(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "gopher",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_gopher(url, &d, Some(&opts))
            },
        );
    }
}

fn do_gopher(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    let timeout = opts
        .and_then(|o| opts_get_u64(o, "timeout_ms"))
        .map(|ms| ms / 1000)
        .unwrap_or(defaults.connect_timeout)
        .max(1);
    let insecure = opts
        .and_then(|o| opts_get_bool(o, "insecure"))
        .unwrap_or(defaults.insecure);

    let r = gopher_probe::probe(url, timeout, insecure).map_err(anyhow_to_rhai)?;
    let content_str = String::from_utf8_lossy(&r.content).into_owned();

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("tls".into(), r.tls.into());
    out.insert("selector".into(), r.selector.into());
    out.insert(
        "item_type".into(),
        match r.item_type { Some(c) => c.to_string().into(), None => Dynamic::UNIT },
    );
    out.insert("connect_ms".into(), r.connect_ms.into());
    out.insert("content".into(), content_str.into());
    out.insert("bytes".into(), Dynamic::from(r.content));
    Ok(out)
}
