//! `tftp(url)` / `tftp(url, opts)` — RFC 1350 UDP read.
//!
//! opts: #{ blksize (int, default 512), timeout_ms (int) }.
//! Returns: #{ host, port, filename, blksize, bytes, connect_ms }.

use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use crate::tftp_probe;
use rhai::{Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("tftp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_tftp(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "tftp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_tftp(url, &d, Some(&opts))
            },
        );
    }
}

fn do_tftp(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    let blksize = opts
        .and_then(|o| opts_get_u64(o, "blksize"))
        .map(|n| n as usize);
    let timeout = opts
        .and_then(|o| opts_get_u64(o, "timeout_ms"))
        .map(|ms| ms / 1000)
        .unwrap_or(defaults.connect_timeout)
        .max(1);

    let r = tftp_probe::probe(url, timeout, blksize).map_err(anyhow_to_rhai)?;

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("filename".into(), r.filename.into());
    out.insert("blksize".into(), (r.blksize as i64).into());
    out.insert("connect_ms".into(), r.connect_ms.into());
    out.insert("bytes".into(), Dynamic::from(r.bytes));
    Ok(out)
}
