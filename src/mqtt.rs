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

/// Decide which of the three MQTT modes to execute based on flags + URL.
///
/// | -d set | --subscribe set | URL has topic | Mode |
/// |--------|-----------------|---------------|------|
/// | no     | no              | either        | Probe |
/// | yes    | no              | yes           | Publish |
/// | yes    | no              | no            | error |
/// | no     | yes             | either        | Subscribe |
/// | yes    | yes             | —             | error (mutually exclusive) |
pub fn dispatch_mode(args: &Args, cfg: &MqttConfig) -> Result<Mode> {
    let has_data = args.data.is_some();
    let has_subscribe = !args.subscribe.is_empty();

    if has_data && has_subscribe {
        bail!("mqtt: -d/--data and --subscribe are mutually exclusive");
    }
    if has_subscribe {
        return Ok(Mode::Subscribe);
    }
    if has_data {
        if cfg.topic.is_none() {
            bail!("mqtt: publish requires a topic in the URL path (e.g. mqtt://broker/topic)");
        }
        return Ok(Mode::Publish);
    }
    Ok(Mode::Probe)
}

fn generate_client_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let suffix: String = (0..6)
        .map(|_| {
            let n: u8 = rng.gen_range(0..16);
            if n < 10 {
                (b'0' + n) as char
            } else {
                (b'a' + n - 10) as char
            }
        })
        .collect();
    format!("recon-{suffix}")
}

fn parse_version(s: &str) -> Result<MqttVersion> {
    match s.trim() {
        "3" | "3.1.1" | "311" => Ok(MqttVersion::V311),
        "5" | "5.0" => Ok(MqttVersion::V5),
        other => bail!("--mqtt-version must be 3 or 5, got '{other}'"),
    }
}

pub fn run(url: &str, args: &Args) -> Result<()> {
    let cfg = MqttConfig::from_url_and_args(url, args)?;
    let mode = dispatch_mode(args, &cfg)?;
    let version = parse_version(&args.mqtt_version)?;
    let client_id = args.client_id.clone().unwrap_or_else(generate_client_id);

    match mode {
        Mode::Probe => probe(&cfg, version, &client_id, args),
        Mode::Publish => Err(anyhow!("mqtt publish: not yet implemented")),
        Mode::Subscribe => Err(anyhow!("mqtt subscribe: not yet implemented")),
    }
}

fn probe(cfg: &MqttConfig, version: MqttVersion, client_id: &str, args: &Args) -> Result<()> {
    match version {
        MqttVersion::V5 => probe_v5(cfg, client_id, args),
        MqttVersion::V311 => probe_v3(cfg, client_id, args),
    }
}

fn probe_v5(cfg: &MqttConfig, client_id: &str, args: &Args) -> Result<()> {
    use rumqttc::v5::mqttbytes::v5::Packet;
    use rumqttc::v5::{AsyncClient, Event, MqttOptions};
    use std::io::Write;
    use std::time::Duration;

    let mut options = MqttOptions::new(client_id, &cfg.host, cfg.port);
    options.set_keep_alive(Duration::from_secs(args.keepalive.into()));
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
        options.set_credentials(u, p);
    } else if let Some(u) = &cfg.username {
        options.set_credentials(u, "");
    }
    if cfg.tls {
        configure_tls_v5(&mut options, args.insecure)?;
    }
    options.set_connection_timeout(args.timeout);
    options.set_clean_start(true);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for mqtt probe")?;

    rt.block_on(async {
        let (client, mut event_loop) = AsyncClient::new(options, 10);
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                    let mut stdout = std::io::stdout();
                    if args.mqtt_json {
                        emit_probe_json_v5(&mut stdout, cfg, &connack)?;
                    } else {
                        writeln!(stdout, "* Connected to {}:{} (MQTT 5.0)", cfg.host, cfg.port)?;
                        writeln!(stdout, "* TLS: {}", if cfg.tls { "rustls" } else { "none" })?;
                        print_connack_v5(&mut stdout, &connack)?;
                    }
                    let _ = client.disconnect().await;
                    let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
                    return Ok(());
                }
                Ok(_other) => continue,
                Err(e) => return Err(anyhow!("mqtt probe: {e}")),
            }
        }
    })
}

fn emit_probe_json_v5<W: std::io::Write>(
    out: &mut W,
    cfg: &MqttConfig,
    connack: &rumqttc::v5::mqttbytes::v5::ConnAck,
) -> Result<()> {
    use serde_json::{json, Map, Value};
    use rumqttc::v5::mqttbytes::v5::ConnectReturnCode;

    let (code, reason) = match connack.code {
        ConnectReturnCode::Success => (0u8, "Success"),
        ConnectReturnCode::BadUserNamePassword => (0x86u8, "Bad User Name or Password"),
        ConnectReturnCode::NotAuthorized => (0x87u8, "Not Authorized"),
        other => (other as u8, "Other"),
    };

    let mut map = Map::new();
    map.insert("broker_host".into(), json!(cfg.host));
    map.insert("broker_port".into(), json!(cfg.port));
    map.insert("protocol_version".into(), json!("5.0"));
    map.insert("tls".into(), if cfg.tls { json!({"backend": "rustls"}) } else { Value::Null });
    map.insert("connect_reason_code".into(), json!(code));
    map.insert("connect_reason".into(), json!(reason));
    map.insert("session_present".into(), json!(connack.session_present));
    if let Some(props) = &connack.properties {
        if let Some(id) = &props.assigned_client_identifier {
            map.insert("assigned_client_id".into(), json!(id));
        }
        if let Some(ka) = props.server_keep_alive {
            map.insert("server_keep_alive".into(), json!(ka));
        }
        if let Some(q) = props.max_qos {
            map.insert("maximum_qos".into(), json!(q));
        }
        if let Some(ra) = props.retain_available {
            map.insert("retain_available".into(), json!(ra != 0));
        }
        if let Some(mps) = props.max_packet_size {
            map.insert("maximum_packet_size".into(), json!(mps));
        }
        if let Some(tam) = props.topic_alias_max {
            map.insert("topic_alias_maximum".into(), json!(tam));
        }
    }
    writeln!(out, "{}", Value::Object(map))?;
    Ok(())
}

fn emit_probe_json_v3<W: std::io::Write>(
    out: &mut W,
    cfg: &MqttConfig,
    connack: &rumqttc::ConnAck,
) -> Result<()> {
    use serde_json::{json, Map, Value};
    let mut map = Map::new();
    map.insert("broker_host".into(), json!(cfg.host));
    map.insert("broker_port".into(), json!(cfg.port));
    map.insert("protocol_version".into(), json!("3.1.1"));
    map.insert("tls".into(), if cfg.tls { json!({"backend": "rustls"}) } else { Value::Null });
    map.insert("connect_return_code".into(), json!(format!("{:?}", connack.code)));
    map.insert("session_present".into(), json!(connack.session_present));
    writeln!(out, "{}", Value::Object(map))?;
    Ok(())
}

fn print_connack_v5<W: std::io::Write>(
    out: &mut W,
    connack: &rumqttc::v5::mqttbytes::v5::ConnAck,
) -> Result<()> {
    use rumqttc::v5::mqttbytes::v5::ConnectReturnCode;
    let (code, reason) = match connack.code {
        ConnectReturnCode::Success => (0u8, "Success"),
        ConnectReturnCode::BadUserNamePassword => (0x86u8, "Bad User Name or Password"),
        ConnectReturnCode::NotAuthorized => (0x87u8, "Not Authorized"),
        other => (other as u8, "Other"),
    };
    writeln!(out, "< Connect Reason Code: {} ({})", code, reason)?;
    writeln!(out, "< Session Present: {}", connack.session_present)?;
    if let Some(props) = &connack.properties {
        if let Some(id) = &props.assigned_client_identifier {
            writeln!(out, "< Assigned Client Identifier: {id}")?;
        }
        if let Some(ka) = props.server_keep_alive {
            writeln!(out, "< Server Keep Alive: {ka}")?;
        }
        if let Some(q) = props.max_qos {
            writeln!(out, "< Maximum QoS: {q}")?;
        }
        if let Some(ra) = props.retain_available {
            writeln!(out, "< Retain Available: {}", ra != 0)?;
        }
        if let Some(mps) = props.max_packet_size {
            writeln!(out, "< Maximum Packet Size: {mps}")?;
        }
        if let Some(tam) = props.topic_alias_max {
            writeln!(out, "< Topic Alias Maximum: {tam}")?;
        }
    }
    Ok(())
}

fn probe_v3(cfg: &MqttConfig, client_id: &str, args: &Args) -> Result<()> {
    use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, NetworkOptions};
    use std::io::Write;
    use std::time::Duration;

    let mut options = MqttOptions::new(client_id, &cfg.host, cfg.port);
    options.set_keep_alive(Duration::from_secs(args.keepalive.into()));
    if let Some(u) = &cfg.username {
        options.set_credentials(u, cfg.password.clone().unwrap_or_default());
    }
    if cfg.tls {
        configure_tls_v3(&mut options, args.insecure)?;
    }
    options.set_clean_session(true);

    let mut net_options = NetworkOptions::new();
    net_options.set_connection_timeout(args.timeout);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for mqtt probe")?;

    rt.block_on(async {
        let (client, mut event_loop) = AsyncClient::new(options, 10);
        event_loop.set_network_options(net_options);
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(connack))) => {
                    let mut stdout = std::io::stdout();
                    if args.mqtt_json {
                        emit_probe_json_v3(&mut stdout, cfg, &connack)?;
                    } else {
                        writeln!(stdout, "* Connected to {}:{} (MQTT 3.1.1)", cfg.host, cfg.port)?;
                        writeln!(stdout, "* TLS: {}", if cfg.tls { "rustls" } else { "none" })?;
                        writeln!(stdout, "< Connect Return Code: {:?}", connack.code)?;
                        writeln!(stdout, "< Session Present: {}", connack.session_present)?;
                    }
                    let _ = client.disconnect().await;
                    let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
                    return Ok(());
                }
                Ok(_other) => continue,
                Err(e) => return Err(anyhow!("mqtt probe: {e}")),
            }
        }
    })
}

fn configure_tls_v5(_options: &mut rumqttc::v5::MqttOptions, _insecure: bool) -> Result<()> {
    bail!("mqtt: mqtts:// TLS not yet implemented; use mqtt:// for now")
}

fn configure_tls_v3(_options: &mut rumqttc::MqttOptions, _insecure: bool) -> Result<()> {
    bail!("mqtt: mqtts:// TLS not yet implemented; use mqtt:// for now")
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

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use clap::Parser;

    fn parse(extra_args: &[&str]) -> Args {
        let mut v = vec!["recon"];
        v.extend_from_slice(extra_args);
        v.push("mqtt://broker/");
        Args::try_parse_from(&v).unwrap()
    }

    #[test]
    fn no_flags_is_probe() {
        let args = parse(&[]);
        let cfg = MqttConfig::from_url("mqtt://broker/").unwrap();
        assert_eq!(dispatch_mode(&args, &cfg).unwrap(), Mode::Probe);
    }

    #[test]
    fn dash_d_with_topic_is_publish() {
        let args = parse(&["-d", "hello"]);
        let cfg = MqttConfig::from_url("mqtt://broker/topic").unwrap();
        assert_eq!(dispatch_mode(&args, &cfg).unwrap(), Mode::Publish);
    }

    #[test]
    fn dash_d_without_topic_errors() {
        let args = parse(&["-d", "hello"]);
        let cfg = MqttConfig::from_url("mqtt://broker/").unwrap();
        let err = dispatch_mode(&args, &cfg).unwrap_err().to_string();
        assert!(err.contains("publish requires a topic"), "got: {err}");
    }

    #[test]
    fn subscribe_flag_is_subscribe() {
        let args = parse(&["--subscribe", "devices/#"]);
        let cfg = MqttConfig::from_url("mqtt://broker/").unwrap();
        assert_eq!(dispatch_mode(&args, &cfg).unwrap(), Mode::Subscribe);
    }

    #[test]
    fn dash_d_and_subscribe_mutually_exclusive() {
        let args = parse(&["-d", "x", "--subscribe", "t"]);
        let cfg = MqttConfig::from_url("mqtt://broker/topic").unwrap();
        let err = dispatch_mode(&args, &cfg).unwrap_err().to_string();
        assert!(err.contains("mutually exclusive"), "got: {err}");
    }
}
