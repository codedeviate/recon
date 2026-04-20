//! `mqtt_pub(url, topic, payload)` and `mqtt_sub(url, topic, max_ms)`
//! script bindings.
//!
//! Wraps the existing CLI `mqtt::run` by synthesising an `Args` struct
//! with the right fields set and reusing the full publish/subscribe
//! codepath. Protocol output (connect banner, received messages for
//! subscribe, etc.) flows to stdout as it does for the CLI; the return
//! map is `#{ ok: true, duration_ms }`. Scripts that need structured
//! per-message data from `mqtt_sub` can capture stdout themselves —
//! collecting messages into the map would require carving the subscribe
//! codepath out of `mqtt.rs`, which is left for a later patch.
//!
//! opts for `mqtt_pub`: `#{ qos, retain, version, username, password,
//! insecure, timeout, client_id, keepalive }`.
//! opts for `mqtt_sub`: same, plus `count` (stop after N messages).

use crate::cli::Args;
use crate::mqtt as core;
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_str, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use clap::Parser;
use rhai::{Engine, EvalAltResult, Map};
use std::time::Instant;

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn(
            "mqtt_pub",
            move |url: &str, payload: &str| -> Result<Map, Box<EvalAltResult>> {
                do_pub(url, payload, &d, None)
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "mqtt_pub",
            move |url: &str, payload: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_pub(url, payload, &d, Some(&opts))
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "mqtt_sub",
            move |url: &str, max_ms: i64| -> Result<Map, Box<EvalAltResult>> {
                do_sub(url, max_ms, &d, None)
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "mqtt_sub",
            move |url: &str, max_ms: i64, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_sub(url, max_ms, &d, Some(&opts))
            },
        );
    }
}

fn base_args(
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Args, Box<EvalAltResult>> {
    // Parse a minimally-valid Args via a placeholder URL, then overwrite.
    let mut args = Args::try_parse_from(["recon", "mqtt://placeholder"])
        .map_err(|e| format!("mqtt: internal Args bootstrap failed: {e}"))?;
    args.insecure = defaults.insecure;
    args.timeout = defaults.connect_timeout;
    args.user = defaults.user.clone();

    if let Some(o) = opts {
        if let Some(v) = opts_get_str(o, "version") {
            args.mqtt_version = v;
        }
        if let Some(id) = opts_get_str(o, "client_id") {
            args.client_id = Some(id);
        }
        if let Some(k) = opts_get_u64(o, "keepalive") {
            args.keepalive = k as u16;
        }
        if let Some(t) = opts_get_u64(o, "timeout") {
            args.timeout = t;
        }
        if let Some(ins) = opts_get_bool(o, "insecure") {
            args.insecure = ins;
        }
        if let Some(u) = opts_get_str(o, "username") {
            let pass = opts_get_str(o, "password").unwrap_or_default();
            args.user = Some(format!("{u}:{pass}"));
        }
    }
    Ok(args)
}

fn do_pub(
    url: &str,
    payload: &str,
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Map, Box<EvalAltResult>> {
    let mut args = base_args(defaults, opts)?;
    args.data = Some(payload.to_string());
    if let Some(o) = opts {
        if let Some(q) = opts_get_u64(o, "qos") {
            args.qos = q as u8;
        }
        if let Some(r) = opts_get_bool(o, "retain") {
            args.retain = r;
        }
    }
    let t0 = Instant::now();
    core::run(url, &args).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("ok".into(), true.into());
    m.insert(
        "duration_ms".into(),
        (t0.elapsed().as_millis() as i64).into(),
    );
    Ok(m)
}

fn do_sub(
    url: &str,
    max_ms: i64,
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Map, Box<EvalAltResult>> {
    if max_ms <= 0 {
        return Err("mqtt_sub: max_ms must be positive".into());
    }
    let topic = url_topic(url).ok_or_else(|| {
        Box::<EvalAltResult>::from(
            "mqtt_sub: URL must include a topic in the path (mqtt://broker/topic)".to_string(),
        )
    })?;
    let mut args = base_args(defaults, opts)?;
    args.subscribe = vec![topic];
    args.max_time = Some((max_ms as f64) / 1000.0);
    if let Some(o) = opts {
        if let Some(c) = opts_get_u64(o, "count") {
            args.count = Some(c as u32);
        }
    }
    let t0 = Instant::now();
    let result = core::run(url, &args);
    // Treat ProtocolExitCode::OperationTimedOut as "clean end of subscription
    // window" when --max-time was what stopped us. If the script wants to know
    // whether the subscription actually received anything, it can gate on
    // stdout via shell redirection. Other errors still bubble up.
    match result {
        Ok(()) => {}
        Err(e) => {
            let is_time_stop = e
                .chain()
                .any(|c| matches!(c.downcast_ref::<core::ProtocolExitCode>(),
                    Some(core::ProtocolExitCode::OperationTimedOut)));
            if !is_time_stop {
                return Err(anyhow_to_rhai(e));
            }
        }
    }
    let mut m = Map::new();
    m.insert("ok".into(), true.into());
    m.insert(
        "duration_ms".into(),
        (t0.elapsed().as_millis() as i64).into(),
    );
    Ok(m)
}

fn url_topic(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path().trim_start_matches('/');
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}
