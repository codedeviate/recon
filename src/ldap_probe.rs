//! LDAP probe. Opens an anonymous connection, reads the RootDSE
//! (objectClass=* at scope=base), reports namingContexts +
//! supportedLDAPVersion + vendorName/vendorVersion if present.
//!
//! URL grammar: `ldap://host[:port]/` or `ldaps://host[:port]/`.
//! Default ports 389 / 636. Exit 0 on successful query; 7 refused;
//! 28 timed out.

use anyhow::{anyhow, Result};
use ldap3::{LdapConn, LdapConnSettings, Scope, SearchEntry};
use std::time::{Duration, Instant};

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let (scheme, rest) = if let Some(r) = url.strip_prefix("ldaps://") {
        ("ldaps", r)
    } else if let Some(r) = url.strip_prefix("ldap://") {
        ("ldap", r)
    } else {
        return Err(anyhow!("ldap: URL must start with ldap:// or ldaps://"));
    };

    let authority = match rest.find('/') {
        Some(i) => &rest[..i],
        None => rest,
    };
    if authority.is_empty() {
        return Err(anyhow!("ldap: URL missing host"));
    }

    // Build a clean URL tungstenite-style so we don't depend on ldap3's own parser
    // for the path/query; we only query the RootDSE.
    let display_url = format!("{scheme}://{authority}");

    let t0 = Instant::now();
    let settings = LdapConnSettings::new().set_conn_timeout(Duration::from_secs(timeout_secs));
    let mut conn = LdapConn::with_settings(settings, &display_url)
        .map_err(|e| classify_ldap_err(e, authority, "connect"))?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!("Connected to {display_url} in {connect_ms:.1}ms");

    // Anonymous simple bind (empty DN + empty password).
    conn.simple_bind("", "")
        .map_err(|e| classify_ldap_err(e, authority, "bind"))?
        .success()
        .map_err(|e| classify_ldap_err(e, authority, "bind"))?;

    let attrs = vec![
        "namingContexts",
        "supportedLDAPVersion",
        "vendorName",
        "vendorVersion",
        "supportedSASLMechanisms",
    ];
    let (rs, _res) = conn
        .search("", Scope::Base, "(objectClass=*)", attrs)
        .map_err(|e| classify_ldap_err(e, authority, "search"))?
        .success()
        .map_err(|e| classify_ldap_err(e, authority, "search"))?;

    if rs.is_empty() {
        println!("(RootDSE returned no entries)");
    } else {
        for e in rs {
            let entry = SearchEntry::construct(e);
            println!("RootDSE:");
            for (attr, values) in entry.attrs {
                for v in values {
                    println!("  {attr}: {v}");
                }
            }
        }
    }

    let _ = conn.unbind();
    Ok(())
}

fn classify_ldap_err(err: ldap3::LdapError, host: &str, stage: &str) -> anyhow::Error {
    let msg = format!("ldap: {stage} to {host} failed: {err}");
    let s = err.to_string().to_lowercase();
    if s.contains("timed out") || s.contains("timeout") {
        anyhow!(msg).context(crate::mqtt::ProtocolExitCode::OperationTimedOut)
    } else if s.contains("refused") {
        anyhow!(msg).context(crate::mqtt::ProtocolExitCode::CouldntConnect)
    } else if s.contains("invalidcredentials") || s.contains("invalid credentials") {
        anyhow!(msg).context(crate::mqtt::ProtocolExitCode::LoginDenied)
    } else if stage == "connect" {
        anyhow!(msg).context(crate::mqtt::ProtocolExitCode::CouldntConnect)
    } else {
        anyhow!(msg)
    }
}

#[cfg(test)]
mod tests {
    // URL parsing is inlined into run(); validate that bad URLs and
    // empty host are rejected via the public run() entry point.
    use super::*;

    #[test]
    fn rejects_non_ldap_scheme() {
        let err = run("http://example.com/", 5).unwrap_err();
        assert!(err.to_string().contains("must start with ldap"));
    }

    #[test]
    fn rejects_missing_host_ldap() {
        let err = run("ldap:///", 5).unwrap_err();
        assert!(err.to_string().contains("missing host"));
    }

    #[test]
    fn rejects_missing_host_ldaps() {
        let err = run("ldaps:///", 5).unwrap_err();
        assert!(err.to_string().contains("missing host"));
    }
}
