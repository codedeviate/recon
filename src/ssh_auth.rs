use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use ssh2::{CheckResult, KnownHostFileKind, Session};

use crate::cli::Args;

/// Returns (ssh_user, optional_password).
/// Priority for user:  user_from_url > -u flag > $USER / $LOGNAME
/// Priority for pass:  --ssh-pass   > -u :pass suffix
pub fn resolve_credentials(user_from_url: &str, args: &Args) -> (String, Option<String>) {
    let user = if !user_from_url.is_empty() {
        user_from_url.to_string()
    } else if let Some(up) = &args.user {
        up.split(':').next().unwrap_or(up).to_string()
    } else {
        std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    };

    let password = args.ssh_pass.clone().or_else(|| {
        args.user
            .as_ref()
            .and_then(|up| up.split_once(':').map(|(_, p)| p.to_string()))
    });

    (user, password)
}

pub fn verify_host_key(sess: &Session, host: &str, port: u16, insecure: bool) -> Result<()> {
    let (key_bytes, _key_type) = sess
        .host_key()
        .ok_or_else(|| anyhow!("Server did not present a host key"))?;

    if insecure {
        return Ok(());
    }

    let known_hosts_path = home_dir().join(".ssh").join("known_hosts");
    if !known_hosts_path.exists() {
        eprintln!(
            "warning: ~/.ssh/known_hosts not found — host key not verified.\n\
             Run `ssh {}` once to accept the key, or pass --insecure to silence this.",
            host
        );
        return Ok(());
    }

    let mut kh = sess
        .known_hosts()
        .context("Failed to initialise known_hosts")?;

    kh.read_file(&known_hosts_path, KnownHostFileKind::OpenSSH)
        .with_context(|| format!("Failed to read {}", known_hosts_path.display()))?;

    match kh.check_port(host, port, key_bytes) {
        CheckResult::Match => Ok(()),
        CheckResult::Mismatch => Err(anyhow!(
            "SSH host key MISMATCH for {host}:{port} — possible MITM attack.\n  \
             If the server was reinstalled, remove the old entry from ~/.ssh/known_hosts.\n  \
             Use --insecure to skip host key checking."
        )),
        CheckResult::NotFound => Err(anyhow!(
            "SSH host key for {host}:{port} is not in ~/.ssh/known_hosts.\n  \
             Connect once with `ssh {host}` to accept the key, or run:\n  \
               ssh-keyscan -p {port} {host} >> ~/.ssh/known_hosts\n  \
             Use --insecure to skip host key checking."
        )),
        CheckResult::Failure => Err(anyhow!(
            "SSH host key check failed for {host}:{port} (libssh2 internal error)"
        )),
    }
}

pub fn authenticate(
    sess: &Session,
    user: &str,
    args: &Args,
    password: Option<&str>,
) -> Result<()> {
    // 1. SSH agent
    if try_agent_auth(sess, user) {
        return Ok(());
    }

    // 2. Explicit key from --ssh-key
    if let Some(key_path) = &args.ssh_key {
        let pubkey = args.ssh_pubkey.as_deref();
        let passphrase = args.ssh_pass.as_deref();
        if sess
            .userauth_pubkey_file(user, pubkey, key_path, passphrase)
            .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    // 3. Default key files
    let ssh_dir = home_dir().join(".ssh");
    for key_name in &["id_ed25519", "id_ecdsa", "id_rsa", "id_dsa"] {
        let priv_path = ssh_dir.join(key_name);
        if !priv_path.exists() {
            continue;
        }
        let passphrase = args.ssh_pass.as_deref();
        // pubkey: None — libssh2 derives it from the private key file
        if sess
            .userauth_pubkey_file(user, None, &priv_path, passphrase)
            .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    // 4. Password auth
    if let Some(pass) = password {
        sess.userauth_password(user, pass)
            .context("SSH password authentication failed")?;
        if sess.authenticated() {
            return Ok(());
        }
    }

    Err(anyhow!(
        "All SSH authentication methods exhausted for user '{user}'.\n  \
         Tried: agent, default key files (~/.ssh/id_ed25519 etc.), password.\n  \
         Provide a key with --ssh-key or a password with --ssh-pass."
    ))
}

pub fn try_agent_auth(sess: &Session, user: &str) -> bool {
    let mut agent = match sess.agent() {
        Ok(a) => a,
        Err(_) => return false,
    };
    if agent.connect().is_err() {
        return false;
    }
    if agent.list_identities().is_err() {
        return false;
    }
    let identities = match agent.identities() {
        Ok(ids) => ids,
        Err(_) => return false,
    };
    for identity in &identities {
        if agent.userauth(user, identity).is_ok() && sess.authenticated() {
            return true;
        }
    }
    false
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn make_args(user: Option<&str>, ssh_pass: Option<&str>) -> crate::cli::Args {
        let mut argv = vec!["recon", "dummy"];
        if let Some(u) = user { argv.extend(&["--user", u]); }
        if let Some(p) = ssh_pass { argv.extend(&["--ssh-pass", p]); }
        crate::cli::Args::parse_from(argv)
    }

    #[test]
    fn resolve_user_from_url() {
        let args = make_args(None, None);
        let (user, pass) = resolve_credentials("alice", &args);
        assert_eq!(user, "alice");
        assert!(pass.is_none());
    }

    #[test]
    fn resolve_user_from_flag() {
        let args = make_args(Some("bob"), None);
        let (user, pass) = resolve_credentials("", &args);
        assert_eq!(user, "bob");
        assert!(pass.is_none());
    }

    #[test]
    fn resolve_user_and_pass_from_flag() {
        let args = make_args(Some("bob:hunter2"), None);
        let (user, pass) = resolve_credentials("", &args);
        assert_eq!(user, "bob");
        assert_eq!(pass.as_deref(), Some("hunter2"));
    }

    #[test]
    fn url_user_overrides_flag_user() {
        let args = make_args(Some("bob"), None);
        let (user, _) = resolve_credentials("alice", &args);
        assert_eq!(user, "alice");
    }

    #[test]
    fn ssh_pass_overrides_flag_pass() {
        let argv = vec!["recon", "dummy", "--user", "bob:wrong", "--ssh-pass", "correct"];
        let args = crate::cli::Args::parse_from(argv);
        let (_, pass) = resolve_credentials("", &args);
        assert_eq!(pass.as_deref(), Some("correct"));
    }
}
