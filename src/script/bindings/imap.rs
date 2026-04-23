//! `imap(url)` / `imap(url, opts)` — IMAP probe / examine / fetch.
//!
//! opts: #{ user, pass, insecure (bool), peek (bool) }.
//! Returns: #{ host, port, tls, capabilities: Array, mailbox?: String,
//! exists?: int, recent?: int, mailboxes?: Array, body?: Blob }.

use crate::imap_probe::{self, ImapArgs};
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_str};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("imap", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_imap(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "imap",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_imap(url, &d, Some(&opts))
            },
        );
    }
}

fn do_imap(url: &str, defaults: &ScriptDefaults, opts: Option<&Map>) -> Result<Map, Box<EvalAltResult>> {
    let user = opts.and_then(|o| opts_get_str(o, "user"));
    let pass = opts.and_then(|o| opts_get_str(o, "pass"));
    let insecure = opts
        .and_then(|o| opts_get_bool(o, "insecure"))
        .unwrap_or(defaults.insecure);
    let peek = opts.and_then(|o| opts_get_bool(o, "peek")).unwrap_or(false);

    let iargs = ImapArgs {
        user: user.as_deref(),
        pass: pass.as_deref(),
        insecure,
        peek,
    };
    let r = imap_probe::probe(url, &iargs).map_err(anyhow_to_rhai)?;

    let mut out = Map::new();
    out.insert("host".into(), r.host.into());
    out.insert("port".into(), (r.port as i64).into());
    out.insert("tls".into(), r.tls.into());
    let caps: Array = r.capabilities.into_iter().map(Dynamic::from).collect();
    out.insert("capabilities".into(), caps.into());
    out.insert(
        "mailbox".into(),
        match r.mailbox { Some(s) => s.into(), None => Dynamic::UNIT },
    );
    out.insert(
        "exists".into(),
        match r.exists { Some(n) => (n as i64).into(), None => Dynamic::UNIT },
    );
    out.insert(
        "recent".into(),
        match r.recent { Some(n) => (n as i64).into(), None => Dynamic::UNIT },
    );
    out.insert(
        "mailboxes".into(),
        match r.mailboxes {
            Some(list) => {
                let a: Array = list.into_iter().map(Dynamic::from).collect();
                a.into()
            }
            None => Dynamic::UNIT,
        },
    );
    out.insert(
        "uid".into(),
        match r.uid { Some(n) => (n as i64).into(), None => Dynamic::UNIT },
    );
    out.insert(
        "body".into(),
        match r.body { Some(b) => Dynamic::from(b), None => Dynamic::UNIT },
    );
    Ok(out)
}
