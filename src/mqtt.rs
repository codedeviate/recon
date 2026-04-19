//! MQTT client: probe, publish, subscribe.
//!
//! Dispatched from `main.rs` on `mqtt://` and `mqtts://` URLs. Three modes
//! gated by CLI flags: probe (default), publish (with `-d` + topic in URL),
//! subscribe (with `--subscribe <filter>`).

use anyhow::{anyhow, bail, Context, Result};

use crate::cli::Args;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttVersion {
    V311,
    V5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Probe,
    Publish,
    Subscribe,
}

#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Topic from the URL path, if any. Used for publish mode.
    pub topic: Option<String>,
}

impl MqttConfig {
    /// Parse just the URL — no arg-derived overrides. `from_url_and_args`
    /// layers `-u` and other flags on top.
    pub fn from_url(url_str: &str) -> Result<Self> {
        let parsed = url::Url::parse(url_str)
            .with_context(|| format!("malformed mqtt URL: {url_str}"))?;

        let tls = match parsed.scheme() {
            "mqtt" => false,
            "mqtts" => true,
            other => bail!("unsupported scheme for mqtt URL: {other} (expected mqtt or mqtts)"),
        };

        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("mqtt URL missing host: {url_str}"))?
            .to_string();

        let port = parsed.port().unwrap_or(if tls { 8883 } else { 1883 });

        let u = parsed.username();
        let username = (!u.is_empty()).then(|| u.to_string());
        let password = parsed.password().map(|p| p.to_string());

        // Path: strip leading '/'; empty → None
        let path = parsed.path().trim_start_matches('/');
        let topic = if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        };

        Ok(MqttConfig {
            host,
            port,
            tls,
            username,
            password,
            topic,
        })
    }

    /// Parse URL and apply CLI-arg overrides (currently: `-u user:pass`).
    pub fn from_url_and_args(url_str: &str, args: &Args) -> Result<Self> {
        let mut cfg = Self::from_url(url_str)?;
        if let Some(user_pass) = &args.user {
            let (u, p) = user_pass
                .split_once(':')
                .map(|(u, p)| (u.to_string(), Some(p.to_string())))
                .unwrap_or((user_pass.clone(), None));
            cfg.username = Some(u);
            cfg.password = p;
        }
        Ok(cfg)
    }
}

pub fn run(_url: &str, _args: &Args) -> Result<()> {
    Err(anyhow!("mqtt: not yet implemented"))
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[test]
    fn mqtt_default_port_1883() {
        let cfg = MqttConfig::from_url("mqtt://broker.example.com/").unwrap();
        assert_eq!(cfg.host, "broker.example.com");
        assert_eq!(cfg.port, 1883);
        assert!(!cfg.tls);
    }

    #[test]
    fn mqtts_default_port_8883() {
        let cfg = MqttConfig::from_url("mqtts://broker.example.com/").unwrap();
        assert_eq!(cfg.port, 8883);
        assert!(cfg.tls);
    }

    #[test]
    fn explicit_port_wins() {
        let cfg = MqttConfig::from_url("mqtt://broker.example.com:2000/").unwrap();
        assert_eq!(cfg.port, 2000);
    }

    #[test]
    fn url_userinfo_extracted() {
        let cfg = MqttConfig::from_url("mqtt://alice:s3cr3t@broker/topic").unwrap();
        assert_eq!(cfg.username.as_deref(), Some("alice"));
        assert_eq!(cfg.password.as_deref(), Some("s3cr3t"));
    }

    #[test]
    fn topic_from_path() {
        let cfg = MqttConfig::from_url("mqtt://broker/devices/fan/state").unwrap();
        assert_eq!(cfg.topic.as_deref(), Some("devices/fan/state"));
    }

    #[test]
    fn empty_path_means_no_topic() {
        let cfg = MqttConfig::from_url("mqtt://broker/").unwrap();
        assert!(cfg.topic.is_none());
        let cfg2 = MqttConfig::from_url("mqtt://broker").unwrap();
        assert!(cfg2.topic.is_none());
    }

    #[test]
    fn malformed_url_errors() {
        assert!(MqttConfig::from_url("not-a-url").is_err());
        assert!(MqttConfig::from_url("http://broker/").is_err()); // wrong scheme
    }

    #[test]
    fn dash_u_overrides_url_userinfo() {
        use clap::Parser;
        let args = Args::try_parse_from(["recon", "mqtt://ignored:bad@b/", "-u", "real:pw"]).unwrap();
        let cfg = MqttConfig::from_url_and_args("mqtt://ignored:bad@b/", &args).unwrap();
        assert_eq!(cfg.username.as_deref(), Some("real"));
        assert_eq!(cfg.password.as_deref(), Some("pw"));
    }
}
