//! `sftp(url)` / `sftp(url, opts)` — SSH-backed file transfer probe.
//!
//! opts: #{ insecure (bool), timeout_ms (int), ssh_key (path) }.
//! Additional auth params (-u user:pass) are resolved from URL userinfo
//! or CLI defaults; scripts embed credentials in the URL.
//!
//! Returns: #{ host, port, user, connect_ms, path,
//! mode: "list" | "retrieve",
//! listing?: Array<#{ name, size, is_dir, mode }>, bytes?: Blob }.

use crate::cli::Args;
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_str, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use crate::sftp_probe::{self, SftpMode};
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("sftp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_sftp(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "sftp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_sftp(url, &d, Some(&opts))
            },
        );
    }
}

fn do_sftp(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    use clap::Parser;
    let mut args = Args::try_parse_from(["recon", "sftp://placeholder/"])
        .map_err(|e| format!("sftp: internal args init: {e}"))?;
    args.insecure = defaults.insecure;
    args.timeout = defaults.connect_timeout;
    args.user = defaults.user.clone();
    if let Some(o) = opts {
        if let Some(b) = opts_get_bool(o, "insecure") {
            args.insecure = b;
        }
        if let Some(ms) = opts_get_u64(o, "timeout_ms") {
            args.timeout = (ms / 1000).max(1);
        }
        if let Some(k) = opts_get_str(o, "ssh_key") {
            args.ssh_key = Some(std::path::PathBuf::from(k));
        }
    }

    let r = sftp_probe::probe(url, &args).map_err(anyhow_to_rhai)?;

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("user".into(), r.user.into());
    out.insert("connect_ms".into(), r.connect_ms.into());
    out.insert("path".into(), r.path.into());
    match r.mode {
        SftpMode::List(entries) => {
            out.insert("mode".into(), "list".into());
            let arr: Array = entries
                .into_iter()
                .map(|e| {
                    let mut m = Map::new();
                    m.insert("name".into(), e.name.into());
                    m.insert("size".into(), (e.size as i64).into());
                    m.insert("is_dir".into(), e.is_dir.into());
                    m.insert(
                        "mode".into(),
                        match e.mode { Some(m) => (m as i64).into(), None => Dynamic::UNIT },
                    );
                    Dynamic::from(m)
                })
                .collect();
            out.insert("listing".into(), arr.into());
        }
        SftpMode::Retrieve(bytes) => {
            out.insert("mode".into(), "retrieve".into());
            out.insert("bytes".into(), Dynamic::from(bytes));
        }
    }
    Ok(out)
}
