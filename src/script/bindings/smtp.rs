//! `smtp(url)` / `smtp(url, opts)` script binding. Returns a Map with
//! the capability survey + optional send-result of the SMTP probe.
//!
//! Opts map mirrors the CLI flags (lowercase snake_case):
//!   mail_from, mail_to (Array), mail_subject, mail_body, mail_header
//!   (Array), smtp_auth, smtp_helo, no_starttls, dkim_key,
//!   dkim_selector, dkim_domain, insecure, timeout_ms.

use crate::cli::Args;
use crate::script::convert::{anyhow_to_rhai, err, opts_clone_array, opts_get_bool, opts_get_str, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use crate::smtp_probe;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use std::path::PathBuf;

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("smtp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_smtp(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "smtp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_smtp(url, &d, Some(&opts))
            },
        );
    }
}

fn do_smtp(
    url: &str,
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Map, Box<EvalAltResult>> {
    let args = build_args(defaults, opts)?;
    let r = smtp_probe::probe(url, &args).map_err(anyhow_to_rhai)?;

    let mut result = Map::new();
    result.insert("host".into(), r.host.into());
    result.insert("port".into(), (r.port as i64).into());
    result.insert("tls".into(), r.tls.into());
    result.insert("connect_ms".into(), r.connect_ms.into());
    result.insert("banner".into(), r.banner.into());

    let caps: Array = r
        .capabilities
        .iter()
        .map(|c| Dynamic::from(c.clone()))
        .collect();
    result.insert("capabilities".into(), caps.into());

    let auth: Array = r
        .auth_methods
        .iter()
        .map(|m| Dynamic::from(m.clone()))
        .collect();
    result.insert("auth_methods".into(), auth.into());

    result.insert(
        "starttls_ok".into(),
        match r.starttls_ok {
            Some(b) => b.into(),
            None => Dynamic::UNIT,
        },
    );

    match r.send_result {
        Some(s) => {
            let mut send = Map::new();
            send.insert("code".into(), (s.code as i64).into());
            send.insert("response".into(), s.response.into());
            send.insert(
                "queued_message_id".into(),
                match s.queued_message_id {
                    Some(id) => id.into(),
                    None => Dynamic::UNIT,
                },
            );
            send.insert("dkim_signed".into(), s.dkim_signed.into());
            result.insert("send_result".into(), send.into());
        }
        None => {
            result.insert("send_result".into(), Dynamic::UNIT);
        }
    }

    Ok(result)
}

fn build_args(
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Args, Box<EvalAltResult>> {
    use clap::Parser;
    // Parse a trivial command line, then fill in what we need.
    let mut args = Args::try_parse_from(["recon", "smtp://placeholder/"])
        .map_err(|e| err(format!("smtp: args init: {e}")))?;
    args.insecure = defaults.insecure;
    args.timeout = defaults.connect_timeout;

    let Some(o) = opts else { return Ok(args); };

    if let Some(s) = opts_get_str(o, "mail_from") {
        args.mail_from = Some(s);
    }
    if let Some(arr) = opts_clone_array(o, "mail_to") {
        args.mail_to = arr
            .into_iter()
            .filter_map(|v| v.try_cast::<String>())
            .collect();
    }
    if let Some(s) = opts_get_str(o, "mail_subject") {
        args.mail_subject = Some(s);
    }
    if let Some(s) = opts_get_str(o, "mail_body") {
        args.mail_body = Some(s);
    }
    if let Some(arr) = opts_clone_array(o, "mail_header") {
        args.mail_header = arr
            .into_iter()
            .filter_map(|v| v.try_cast::<String>())
            .collect();
    }
    if let Some(s) = opts_get_str(o, "smtp_auth") {
        args.smtp_auth = Some(s);
    }
    if let Some(s) = opts_get_str(o, "smtp_helo") {
        args.smtp_helo = Some(s);
    }
    if let Some(b) = opts_get_bool(o, "no_starttls") {
        args.no_starttls = b;
    }
    if let Some(s) = opts_get_str(o, "dkim_key") {
        args.dkim_key = Some(PathBuf::from(s));
    }
    if let Some(s) = opts_get_str(o, "dkim_selector") {
        args.dkim_selector = Some(s);
    }
    if let Some(s) = opts_get_str(o, "dkim_domain") {
        args.dkim_domain = Some(s);
    }
    if let Some(b) = opts_get_bool(o, "insecure") {
        args.insecure = b;
    }
    if let Some(ms) = opts_get_u64(o, "timeout_ms") {
        args.timeout = (ms / 1000).max(1);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn defaults() -> ScriptDefaults {
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        ScriptDefaults::from_args(&args)
    }

    #[test]
    fn build_args_defaults_are_inherited() {
        let args = build_args(&defaults(), None).unwrap();
        assert_eq!(args.timeout, 30);
        assert!(!args.insecure);
    }

    #[test]
    fn build_args_maps_opts() {
        let mut o = Map::new();
        o.insert("mail_from".into(), "a@b.com".into());
        let mut tos = rhai::Array::new();
        tos.push("x@y.com".into());
        tos.push("z@q.com".into());
        o.insert("mail_to".into(), tos.into());
        o.insert("smtp_auth".into(), "user:pass".into());
        o.insert("no_starttls".into(), true.into());

        let args = build_args(&defaults(), Some(&o)).unwrap();
        assert_eq!(args.mail_from.as_deref(), Some("a@b.com"));
        assert_eq!(args.mail_to.len(), 2);
        assert_eq!(args.smtp_auth.as_deref(), Some("user:pass"));
        assert!(args.no_starttls);
    }
}
