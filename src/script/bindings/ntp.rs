//! `ntp(url)` script binding. Wraps the SNTPv4 probe.
//!
//! Returns `#{ host, port, stratum, precision, poll_interval,
//! ref_id, reference_ts, offset_ms, delay_ms }`.

use crate::ntp_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("ntp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_ntp(url, d.connect_timeout)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "ntp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_ntp(url, timeout)
            },
        );
    }
}

fn do_ntp(url: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    // ntp:// scheme required; auto-prepend for bare hosts.
    let full_url = if url.contains("://") {
        url.to_string()
    } else {
        format!("ntp://{url}")
    };
    let p = ntp_probe::probe(&full_url, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("host".into(), p.host.into());
    m.insert("port".into(), (p.port as i64).into());
    m.insert("stratum".into(), (p.stratum as i64).into());
    m.insert("precision".into(), (p.precision as i64).into());
    m.insert("poll_interval".into(), (p.poll_interval as i64).into());
    m.insert("ref_id".into(), p.ref_id_formatted.into());
    m.insert("reference_ts".into(), p.reference_ts.into());
    m.insert("offset_ms".into(), (p.offset_secs * 1000.0).into());
    m.insert("delay_ms".into(), (p.delay_secs.abs() * 1000.0).into());
    Ok(m)
}
