//! `ws(url)` / `wss(url)` script binding. Opens a WebSocket, sends a
//! Ping with a known nonce, waits for the matching Pong, closes cleanly.
//! Returns `#{ host, port, scheme, connect_ms, handshake_ms,
//! http_status, headers: #{ ... }, pong_nonce_matched, ping_ms }`.

use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use crate::ws_probe;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("ws", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_ws(url, d.connect_timeout)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "ws",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_ws(url, timeout)
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn("wss", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_ws(url, d.connect_timeout)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "wss",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_ws(url, timeout)
            },
        );
    }
}

fn do_ws(url: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let r = ws_probe::probe(url, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("host".into(), r.host.into());
    m.insert("port".into(), (r.port as i64).into());
    m.insert("scheme".into(), r.scheme.to_string().into());
    m.insert("connect_ms".into(), r.connect_ms.into());
    m.insert("handshake_ms".into(), r.handshake_ms.into());
    m.insert("http_status".into(), (r.http_status as i64).into());
    let mut headers_map = Map::new();
    for h in &r.headers {
        headers_map.insert(h.name.as_str().into(), h.value.clone().into());
    }
    m.insert("headers".into(), headers_map.into());
    m.insert("pong_nonce_matched".into(), r.pong_nonce_matched.into());
    m.insert("ping_ms".into(), r.ping_round_trip_ms.into());
    Ok(m)
}
