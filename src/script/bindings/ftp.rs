//! `ftp(url)` / `ftp(url, opts)` script binding.
//!
//! opts: #{ user, pass, passive (bool, default true), implicit_tls
//! (bool, default false), insecure (bool), timeout_ms (int),
//! list_only (bool), quote (str or Array<str>),
//! ftp_skip_pasv_ip (bool), disable_epsv (bool),
//! disable_eprt (bool), ftp_pasv (bool), verbose (int) }.
//!
//! Returns: #{ host, port, tls, user, connect_ms, welcome, pwd,
//! mode: "list" | "retrieve", listing?: Array<String>, bytes?: Blob }.

use crate::ftp_probe::{self, FtpArgs, FtpMode};
use crate::script::convert::{
    anyhow_to_rhai, opts_clone_array, opts_get_bool, opts_get_str, opts_get_u64,
};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("ftp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_ftp(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "ftp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_ftp(url, &d, Some(&opts))
            },
        );
    }
}

fn do_ftp(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    let user = opts.and_then(|o| opts_get_str(o, "user"));
    let pass = opts.and_then(|o| opts_get_str(o, "pass"));
    let passive = opts.and_then(|o| opts_get_bool(o, "passive")).unwrap_or(true);
    let implicit_tls = opts.and_then(|o| opts_get_bool(o, "implicit_tls")).unwrap_or(false);
    let insecure = opts
        .and_then(|o| opts_get_bool(o, "insecure"))
        .unwrap_or(defaults.insecure);
    let timeout_ms = opts
        .and_then(|o| opts_get_u64(o, "timeout_ms"))
        .map(|ms| ms / 1000)
        .unwrap_or(defaults.connect_timeout);
    let list_only = opts.and_then(|o| opts_get_bool(o, "list_only")).unwrap_or(false);
    // quote: accept a single string or an array of strings
    let quote: Vec<String> = opts
        .and_then(|o| {
            if let Some(s) = opts_get_str(o, "quote") {
                Some(vec![s])
            } else {
                opts_clone_array(o, "quote").map(|arr| {
                    arr.into_iter()
                        .filter_map(|v| v.into_string().ok())
                        .collect()
                })
            }
        })
        .unwrap_or_default();
    let ftp_skip_pasv_ip = opts.and_then(|o| opts_get_bool(o, "ftp_skip_pasv_ip")).unwrap_or(false);
    let disable_epsv = opts.and_then(|o| opts_get_bool(o, "disable_epsv")).unwrap_or(false);
    let disable_eprt = opts.and_then(|o| opts_get_bool(o, "disable_eprt")).unwrap_or(false);
    let ftp_pasv = opts.and_then(|o| opts_get_bool(o, "ftp_pasv")).unwrap_or(false);
    let verbose = opts
        .and_then(|o| opts_get_u64(o, "verbose"))
        .map(|v| v.min(255) as u8)
        .unwrap_or(0);

    let fargs = FtpArgs {
        user: user.as_deref(),
        pass: pass.as_deref(),
        passive,
        implicit_tls,
        insecure,
        timeout_secs: timeout_ms.max(1),
        list_only,
        quote,
        ftp_skip_pasv_ip,
        disable_epsv,
        disable_eprt,
        ftp_pasv,
        verbose,
    };
    let r = ftp_probe::probe(url, &fargs).map_err(anyhow_to_rhai)?;

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("tls".into(), r.tls.into());
    out.insert("user".into(), r.user.into());
    out.insert("connect_ms".into(), r.connect_ms.into());
    out.insert(
        "welcome".into(),
        match r.welcome { Some(s) => s.into(), None => Dynamic::UNIT },
    );
    out.insert(
        "pwd".into(),
        match r.pwd { Some(s) => s.into(), None => Dynamic::UNIT },
    );
    match r.mode {
        FtpMode::List(entries) => {
            out.insert("mode".into(), "list".into());
            let a: Array = entries.into_iter().map(Dynamic::from).collect();
            out.insert("listing".into(), a.into());
        }
        FtpMode::Retrieve(bytes) => {
            out.insert("mode".into(), "retrieve".into());
            out.insert("bytes".into(), Dynamic::from(bytes));
        }
    }
    Ok(out)
}
