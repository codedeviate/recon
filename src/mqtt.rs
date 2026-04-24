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

/// Context tag attached to anyhow errors returned from protocol operations,
/// used by `main.rs::exit_code_for_http_error` to map to curl-compatible
/// exit codes. Attached via `.context(ProtocolExitCode::...)` on the
/// error paths; `main` recovers it via `downcast_ref::<ProtocolExitCode>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolExitCode {
    CouldntConnect = 7,
    OperationTimedOut = 28,
    LoginDenied = 67,
    Interrupted = 130,
}

impl std::fmt::Display for ProtocolExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CouldntConnect => write!(f, "exit-7"),
            Self::OperationTimedOut => write!(f, "exit-28"),
            Self::LoginDenied => write!(f, "exit-67"),
            Self::Interrupted => write!(f, "exit-130"),
        }
    }
}

impl std::error::Error for ProtocolExitCode {}

/// Classify a v3 rumqttc ConnectionError into an optional exit-code tag.
///
/// - `Io(kind)` with typical connect-failure kinds → CouldntConnect (7)
/// - `NetworkTimeout` → OperationTimedOut (28)
/// - `ConnectionRefused(BadUserNamePassword | NotAuthorized)` → LoginDenied (67)
///
/// Note: in rumqttc 0.24, a non-Success ConnAck is surfaced through the
/// `ConnectionError::ConnectionRefused(code)` variant — the consumer never
/// sees the raw ConnAck packet via `poll()` for auth failures — so we do
/// the auth classification here rather than inspecting ConnAck packets.
fn classify_connection_error_v3(e: &rumqttc::ConnectionError) -> Option<ProtocolExitCode> {
    use rumqttc::{ConnectReturnCode, ConnectionError};
    match e {
        ConnectionError::Io(io_err) if is_connect_io_kind(io_err.kind()) => {
            Some(ProtocolExitCode::CouldntConnect)
        }
        ConnectionError::NetworkTimeout | ConnectionError::FlushTimeout => {
            Some(ProtocolExitCode::OperationTimedOut)
        }
        ConnectionError::ConnectionRefused(
            ConnectReturnCode::BadUserNamePassword | ConnectReturnCode::NotAuthorized,
        ) => Some(ProtocolExitCode::LoginDenied),
        _ => None,
    }
}

/// Classify a v5 rumqttc ConnectionError — see `classify_connection_error_v3`.
/// The v5 variant uses `Timeout(Elapsed)` (not `NetworkTimeout`) and the v5
/// `ConnectReturnCode` has a much larger variant set, but the two login
/// failures have the same names.
fn classify_connection_error_v5(e: &rumqttc::v5::ConnectionError) -> Option<ProtocolExitCode> {
    use rumqttc::v5::mqttbytes::v5::ConnectReturnCode;
    use rumqttc::v5::ConnectionError;
    match e {
        ConnectionError::Io(io_err) if is_connect_io_kind(io_err.kind()) => {
            Some(ProtocolExitCode::CouldntConnect)
        }
        ConnectionError::Timeout(_) => Some(ProtocolExitCode::OperationTimedOut),
        ConnectionError::ConnectionRefused(
            ConnectReturnCode::BadUserNamePassword | ConnectReturnCode::NotAuthorized,
        ) => Some(ProtocolExitCode::LoginDenied),
        _ => None,
    }
}

/// io::ErrorKind values that indicate "couldn't open / maintain a TCP
/// connection to the broker". Used by both v3 and v5 classifiers.
fn is_connect_io_kind(kind: std::io::ErrorKind) -> bool {
    use std::io::ErrorKind;
    matches!(
        kind,
        ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::TimedOut
            | ErrorKind::NotFound
            | ErrorKind::NotConnected
            | ErrorKind::HostUnreachable
            | ErrorKind::NetworkUnreachable
            | ErrorKind::AddrNotAvailable
    )
}

/// Build an anyhow error for a v3 connect-phase failure, tagged with the
/// matching ProtocolExitCode (if any). Used at the `Err(e) => return ...`
/// arms of the ConnAck-wait loops in probe/publish/subscribe.
fn connect_err_v3(phase: &str, e: rumqttc::ConnectionError) -> anyhow::Error {
    let tag = classify_connection_error_v3(&e);
    let err = anyhow!("mqtt {phase}: {e}");
    match tag {
        Some(code) => err.context(code),
        None => err,
    }
}

/// v5 counterpart of `connect_err_v3`.
fn connect_err_v5(phase: &str, e: rumqttc::v5::ConnectionError) -> anyhow::Error {
    let tag = classify_connection_error_v5(&e);
    let err = anyhow!("mqtt {phase}: {e}");
    match tag {
        Some(code) => err.context(code),
        None => err,
    }
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
        Mode::Publish => publish(&cfg, version, &client_id, args),
        Mode::Subscribe => subscribe(&cfg, version, &client_id, args),
    }
}

fn probe(cfg: &MqttConfig, version: MqttVersion, client_id: &str, args: &Args) -> Result<()> {
    match version {
        MqttVersion::V5 => probe_v5(cfg, client_id, args),
        MqttVersion::V311 => probe_v3(cfg, client_id, args),
    }
}

fn publish(cfg: &MqttConfig, version: MqttVersion, client_id: &str, args: &Args) -> Result<()> {
    let topic = cfg.topic.as_deref().ok_or_else(|| {
        anyhow!("mqtt: publish requires a topic in the URL path")
    })?;
    let payload_str = args.data.as_deref().ok_or_else(|| {
        anyhow!("mqtt: publish requires -d/--data")
    })?;
    let payload = crate::client::load_body_from_string(payload_str)?;
    let qos_level = parse_qos_u8(args.qos)?;

    match version {
        MqttVersion::V5 => publish_v5(cfg, client_id, topic, &payload, qos_level, args),
        MqttVersion::V311 => publish_v3(cfg, client_id, topic, &payload, qos_level, args),
    }
}

fn parse_qos_u8(n: u8) -> Result<u8> {
    if n > 2 {
        bail!("--qos must be 0, 1, or 2, got {n}");
    }
    Ok(n)
}

/// Build a current-thread tokio runtime for a one-shot MQTT operation.
/// Each probe/publish/subscribe call builds and drops its own runtime; this
/// keeps the MQTT module sync-on-the-outside while letting rumqttc use tokio
/// internally.
fn build_mqtt_runtime(label: &str) -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .with_context(|| format!("failed to build tokio runtime for mqtt {label}"))
}

/// Assemble a v5 `MqttOptions` from config + args.
fn setup_options_v5(
    cfg: &MqttConfig,
    client_id: &str,
    args: &Args,
) -> Result<rumqttc::v5::MqttOptions> {
    use rumqttc::v5::mqttbytes::v5::{ConnectProperties, LastWill};
    use rumqttc::v5::mqttbytes::QoS;
    use std::time::Duration;

    let mut options = rumqttc::v5::MqttOptions::new(client_id, &cfg.host, cfg.port);
    options.set_keep_alive(Duration::from_secs(args.keepalive.into()));
    if let Some(u) = &cfg.username {
        options.set_credentials(u, cfg.password.clone().unwrap_or_default());
    }
    if cfg.tls {
        configure_tls_v5(&mut options, args.insecure, args)?;
    }
    options.set_connection_timeout(args.timeout);
    options.set_clean_start(args.clean_start);

    // Connect properties — session-expiry, user-properties, enhanced auth.
    let user_properties = parse_user_properties(&args.user_property)?;
    let wants_connect_props = args.session_expiry.is_some()
        || !user_properties.is_empty()
        || args.auth_method.is_some()
        || args.auth_data.is_some();
    if wants_connect_props {
        let mut props = ConnectProperties::new();
        props.session_expiry_interval = args.session_expiry;
        props.user_properties = user_properties;
        props.authentication_method = args.auth_method.clone();
        if let Some(data) = &args.auth_data {
            let bytes = crate::client::load_body_from_string(data)?;
            props.authentication_data = Some(bytes.into());
        }
        options.set_connect_properties(props);
    }

    // Last-will message on unexpected disconnect.
    if let Some(topic) = &args.will_topic {
        let payload = match args.will_payload.as_deref() {
            Some(s) => crate::client::load_body_from_string(s)?,
            None => Vec::new(),
        };
        let qos = match args.will_qos {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            other => bail!("--will-qos must be 0, 1, or 2 (got {other})"),
        };
        let will = LastWill::new(topic.as_str(), payload, qos, args.will_retain, None);
        options.set_last_will(will);
    }

    Ok(options)
}

fn parse_user_properties(specs: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(specs.len());
    for s in specs {
        let (k, v) = s
            .split_once('=')
            .ok_or_else(|| anyhow!("--user-property '{s}' must be KEY=VAL"))?;
        out.push((k.to_string(), v.to_string()));
    }
    Ok(out)
}

/// Build optional PublishProperties from args (content-type, response-topic,
/// correlation-data, user-properties). Returns None when no v5-publish
/// property was set — callers fall back to the non-`_with_properties`
/// publish method to avoid wire-cost for plain publishes.
fn publish_properties(args: &Args) -> Result<Option<rumqttc::v5::mqttbytes::v5::PublishProperties>> {
    let user_properties = parse_user_properties(&args.user_property)?;
    let has_any = args.content_type.is_some()
        || args.response_topic.is_some()
        || args.correlation_data.is_some()
        || !user_properties.is_empty();
    if !has_any {
        return Ok(None);
    }
    let mut p = rumqttc::v5::mqttbytes::v5::PublishProperties {
        payload_format_indicator: None,
        message_expiry_interval: None,
        topic_alias: None,
        response_topic: args.response_topic.clone(),
        correlation_data: match &args.correlation_data {
            Some(s) => Some(crate::client::load_body_from_string(s)?.into()),
            None => None,
        },
        user_properties,
        subscription_identifiers: Vec::new(),
        content_type: args.content_type.clone(),
    };
    // content_type is already set above; avoid `mut` lint noise.
    let _ = &mut p;
    Ok(Some(p))
}

/// Same for subscribe.
fn subscribe_properties(args: &Args) -> Result<Option<rumqttc::v5::mqttbytes::v5::SubscribeProperties>> {
    let user_properties = parse_user_properties(&args.user_property)?;
    if user_properties.is_empty() {
        return Ok(None);
    }
    Ok(Some(rumqttc::v5::mqttbytes::v5::SubscribeProperties {
        id: None,
        user_properties,
    }))
}

/// Assemble a v3 `MqttOptions` + `NetworkOptions` from config + args.
/// v3 puts the connection timeout on `NetworkOptions`, applied via
/// `event_loop.set_network_options(...)` after `AsyncClient::new`.
fn setup_options_v3(
    cfg: &MqttConfig,
    client_id: &str,
    args: &Args,
) -> Result<(rumqttc::MqttOptions, rumqttc::NetworkOptions)> {
    use std::time::Duration;
    let mut options = rumqttc::MqttOptions::new(client_id, &cfg.host, cfg.port);
    options.set_keep_alive(Duration::from_secs(args.keepalive.into()));
    if let Some(u) = &cfg.username {
        options.set_credentials(u, cfg.password.clone().unwrap_or_default());
    }
    if cfg.tls {
        configure_tls_v3(&mut options, args.insecure, args)?;
    }
    options.set_clean_session(true);
    let mut net_options = rumqttc::NetworkOptions::new();
    net_options.set_connection_timeout(args.timeout);
    Ok((options, net_options))
}

fn publish_v5(
    cfg: &MqttConfig,
    client_id: &str,
    topic: &str,
    payload: &[u8],
    qos: u8,
    args: &Args,
) -> Result<()> {
    use rumqttc::v5::mqttbytes::v5::Packet;
    use rumqttc::v5::mqttbytes::QoS;
    use rumqttc::v5::{AsyncClient, Event};
    use std::time::Duration;

    let qos_enum = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => unreachable!("parse_qos_u8 already validated"),
    };

    let options = setup_options_v5(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("publish")?;

    rt.block_on(async {
        let (client, mut event_loop) = AsyncClient::new(options, 10);

        // Wait for ConnAck before publishing.
        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => return Err(connect_err_v5("publish (connect)", e)),
            }
        }

        let pub_props = publish_properties(args)?;
        match pub_props {
            Some(p) => client
                .publish_with_properties(topic, qos_enum, args.retain, payload.to_vec(), p)
                .await
                .map_err(|e| anyhow!("mqtt publish: {e}"))?,
            None => client
                .publish(topic, qos_enum, args.retain, payload.to_vec())
                .await
                .map_err(|e| anyhow!("mqtt publish: {e}"))?,
        }

        if qos == 0 {
            // QoS 0: fire and forget. Give the event loop a tick.
            let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        } else {
            // QoS 1 or 2: wait for PubAck or PubComp.
            let deadline = Duration::from_secs(args.timeout);
            let result = tokio::time::timeout(deadline, async {
                loop {
                    match event_loop.poll().await {
                        Ok(Event::Incoming(Packet::PubAck(_))) => return Ok::<(), anyhow::Error>(()),
                        Ok(Event::Incoming(Packet::PubComp(_))) => return Ok(()),
                        Ok(_) => continue,
                        Err(e) => return Err(anyhow!("mqtt publish (ack): {e}")),
                    }
                }
            }).await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    return Err(anyhow!("mqtt: publish timeout waiting for broker ACK")
                        .context(ProtocolExitCode::OperationTimedOut));
                }
            }
        }

        let _ = client.disconnect().await;
        let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        Ok(())
    })
}

fn publish_v3(
    cfg: &MqttConfig,
    client_id: &str,
    topic: &str,
    payload: &[u8],
    qos: u8,
    args: &Args,
) -> Result<()> {
    use rumqttc::{AsyncClient, Event, Incoming, QoS};
    use std::time::Duration;

    let qos_enum = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => unreachable!("parse_qos_u8 already validated"),
    };

    let (options, net_options) = setup_options_v3(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("publish")?;

    rt.block_on(async {
        let (client, mut event_loop) = AsyncClient::new(options, 10);
        event_loop.set_network_options(net_options);

        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => return Err(connect_err_v3("publish (connect)", e)),
            }
        }

        client.publish(topic, qos_enum, args.retain, payload.to_vec())
            .await
            .map_err(|e| anyhow!("mqtt publish: {e}"))?;

        if qos == 0 {
            let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        } else {
            let deadline = Duration::from_secs(args.timeout);
            let result = tokio::time::timeout(deadline, async {
                loop {
                    match event_loop.poll().await {
                        Ok(Event::Incoming(Incoming::PubAck(_))) => return Ok::<(), anyhow::Error>(()),
                        Ok(Event::Incoming(Incoming::PubComp(_))) => return Ok(()),
                        Ok(_) => continue,
                        Err(e) => return Err(anyhow!("mqtt publish (ack): {e}")),
                    }
                }
            }).await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    return Err(anyhow!("mqtt: publish timeout waiting for broker ACK")
                        .context(ProtocolExitCode::OperationTimedOut));
                }
            }
        }

        let _ = client.disconnect().await;
        let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        Ok(())
    })
}

fn probe_v5(cfg: &MqttConfig, client_id: &str, args: &Args) -> Result<()> {
    use rumqttc::v5::mqttbytes::v5::Packet;
    use rumqttc::v5::{AsyncClient, Event};
    use std::io::Write;
    use std::time::Duration;

    let options = setup_options_v5(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("probe")?;

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
                Err(e) => return Err(connect_err_v5("probe", e)),
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
    // `tls` is always an object (never null) so consumers can safely destructure.
    // When TLS is enabled, Task 11 will add peer_cn / cipher / alpn etc.
    map.insert(
        "tls".into(),
        if cfg.tls {
            json!({"enabled": true, "backend": "rustls"})
        } else {
            json!({"enabled": false})
        },
    );
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
    map.insert(
        "tls".into(),
        if cfg.tls {
            json!({"enabled": true, "backend": "rustls"})
        } else {
            json!({"enabled": false})
        },
    );
    // Debug-formatted string intentionally: MQTT 3.1.1 has a small named
    // variant set (Success, BadUserNamePassword, NotAuthorized, …) and the
    // debug name is more readable for operators than the wire-byte number.
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
    use rumqttc::{AsyncClient, Event, Incoming};
    use std::io::Write;
    use std::time::Duration;

    let (options, net_options) = setup_options_v3(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("probe")?;

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
                Err(e) => return Err(connect_err_v3("probe", e)),
            }
        }
    })
}

fn subscribe(cfg: &MqttConfig, version: MqttVersion, client_id: &str, args: &Args) -> Result<()> {
    let qos_level = parse_qos_u8(args.qos)?;

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let stop_clone = stop.clone();
        ctrlc::set_handler(move || {
            stop_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        })
        .context("failed to install Ctrl-C handler")?;
    }

    match version {
        MqttVersion::V5 => subscribe_v5(cfg, client_id, args, qos_level, stop),
        MqttVersion::V311 => subscribe_v3(cfg, client_id, args, qos_level, stop),
    }
}

fn subscribe_v3(
    cfg: &MqttConfig,
    client_id: &str,
    args: &Args,
    qos: u8,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    use rumqttc::{AsyncClient, Event, Incoming, QoS};
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let qos_enum = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => unreachable!("parse_qos_u8 already validated"),
    };

    let (options, net_options) = setup_options_v3(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("subscribe")?;

    let topics: Vec<String> = args.subscribe.clone();
    let count_limit = args.count;
    let verbose = args.verbose >= 1;
    let mqtt_json = args.mqtt_json;

    rt.block_on(async move {
        let (client, mut event_loop) = AsyncClient::new(options, 100);
        event_loop.set_network_options(net_options);

        // Wait for ConnAck
        loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(());
            }
            match event_loop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => return Err(connect_err_v3("subscribe (connect)", e)),
            }
        }

        for filter in &topics {
            client
                .subscribe(filter, qos_enum)
                .await
                .with_context(|| format!("mqtt subscribe: failed on filter '{filter}'"))?;
        }

        let mut received: u32 = 0;
        let mut stdout = std::io::stdout();
        while !stop.load(Ordering::SeqCst) {
            let event = tokio::time::timeout(Duration::from_millis(200), event_loop.poll()).await;
            let event = match event {
                Err(_) => continue, // timeout tick — loop re-checks `stop`
                Ok(Err(e)) => return Err(anyhow!("mqtt subscribe: {e}")),
                Ok(Ok(ev)) => ev,
            };
            if let Event::Incoming(Incoming::Publish(pub_msg)) = event {
                if mqtt_json {
                    emit_subscribe_json(
                        &mut stdout,
                        &pub_msg.topic,
                        qos_v3_to_u8(pub_msg.qos),
                        pub_msg.retain,
                        &pub_msg.payload,
                    )?;
                } else {
                    emit_subscribe_text(&mut stdout, verbose, &pub_msg.topic, &pub_msg.payload)?;
                }
                received += 1;
                if let Some(n) = count_limit {
                    if received >= n {
                        break;
                    }
                }
            }
        }

        let _ = client.disconnect().await;
        let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        if stop.load(Ordering::SeqCst) {
            return Err(anyhow!("interrupted").context(ProtocolExitCode::Interrupted));
        }
        Ok(())
    })
}

fn subscribe_v5(
    cfg: &MqttConfig,
    client_id: &str,
    args: &Args,
    qos: u8,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    use rumqttc::v5::mqttbytes::v5::Packet;
    use rumqttc::v5::mqttbytes::QoS;
    use rumqttc::v5::{AsyncClient, Event};
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let qos_enum = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => unreachable!("parse_qos_u8 already validated"),
    };

    let options = setup_options_v5(cfg, client_id, args)?;
    let rt = build_mqtt_runtime("subscribe")?;

    let topics: Vec<String> = args.subscribe.clone();
    let count_limit = args.count;
    let verbose = args.verbose >= 1;
    let mqtt_json = args.mqtt_json;

    rt.block_on(async move {
        let (client, mut event_loop) = AsyncClient::new(options, 100);

        loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(());
            }
            match event_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => return Err(connect_err_v5("subscribe (connect)", e)),
            }
        }

        let sub_props = subscribe_properties(args)?;
        for filter in &topics {
            match &sub_props {
                Some(p) => client
                    .subscribe_with_properties(filter.as_str(), qos_enum, p.clone())
                    .await
                    .with_context(|| format!("mqtt subscribe: failed on filter '{filter}'"))?,
                None => client
                    .subscribe(filter.as_str(), qos_enum)
                    .await
                    .with_context(|| format!("mqtt subscribe: failed on filter '{filter}'"))?,
            }
        }

        let mut received: u32 = 0;
        let mut stdout = std::io::stdout();
        while !stop.load(Ordering::SeqCst) {
            let event = tokio::time::timeout(Duration::from_millis(200), event_loop.poll()).await;
            let event = match event {
                Err(_) => continue,
                Ok(Err(e)) => return Err(anyhow!("mqtt subscribe: {e}")),
                Ok(Ok(ev)) => ev,
            };
            if let Event::Incoming(Packet::Publish(pub_msg)) = event {
                // v5: topic is Bytes; convert to str for display
                let topic_str = std::str::from_utf8(&pub_msg.topic)
                    .unwrap_or("<invalid-utf8-topic>");
                if mqtt_json {
                    emit_subscribe_json(
                        &mut stdout,
                        topic_str,
                        qos_v5_to_u8(pub_msg.qos),
                        pub_msg.retain,
                        &pub_msg.payload,
                    )?;
                } else {
                    emit_subscribe_text(&mut stdout, verbose, topic_str, &pub_msg.payload)?;
                }
                received += 1;
                if let Some(n) = count_limit {
                    if received >= n {
                        break;
                    }
                }
            }
        }

        let _ = client.disconnect().await;
        let _ = tokio::time::timeout(Duration::from_millis(500), event_loop.poll()).await;
        if stop.load(Ordering::SeqCst) {
            return Err(anyhow!("interrupted").context(ProtocolExitCode::Interrupted));
        }
        Ok(())
    })
}

fn emit_subscribe_text<W: std::io::Write>(
    out: &mut W,
    verbose: bool,
    topic: &str,
    payload: &[u8],
) -> Result<()> {
    let text = match std::str::from_utf8(payload) {
        Ok(s) => s.to_string(),
        Err(_) => {
            // Non-UTF-8: escape non-printable bytes as \xHH
            let mut s = String::with_capacity(payload.len() * 4);
            for b in payload {
                if *b >= 0x20 && *b < 0x7f {
                    s.push(*b as char);
                } else {
                    s.push_str(&format!("\\x{:02x}", b));
                }
            }
            s
        }
    };
    if verbose {
        writeln!(out, "{topic} {text}")?;
    } else {
        writeln!(out, "{text}")?;
    }
    Ok(())
}

/// Emit one subscribe message as a single JSON object on its own line
/// (NDJSON). Non-UTF-8 payloads are wrapped as `{"base64": "..."}` so the
/// JSON stays well-formed while remaining self-describing.
fn emit_subscribe_json<W: std::io::Write>(
    out: &mut W,
    topic: &str,
    qos: u8,
    retain: bool,
    payload: &[u8],
) -> Result<()> {
    use serde_json::{json, Map, Value};
    let mut map = Map::new();
    map.insert("topic".into(), json!(topic));
    map.insert("qos".into(), json!(qos));
    map.insert("retain".into(), json!(retain));
    let payload_value = match std::str::from_utf8(payload) {
        Ok(s) => json!(s),
        Err(_) => {
            // STANDARD alphabet with padding: round-trips with stdlib base64
            // decoders in jq, Python, Node, etc. — don't switch to URL_SAFE.
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            json!({ "base64": STANDARD.encode(payload) })
        }
    };
    map.insert("payload".into(), payload_value);
    writeln!(out, "{}", Value::Object(map))?;
    // Flush so consumers piping `--mqtt-json | jq` see each message promptly
    // (stdout is block-buffered when piped; would otherwise hold messages).
    out.flush()?;
    Ok(())
}

/// MQTT 3.1.1 QoS → wire-level u8 for JSON emission.
fn qos_v3_to_u8(qos: rumqttc::QoS) -> u8 {
    match qos {
        rumqttc::QoS::AtMostOnce => 0,
        rumqttc::QoS::AtLeastOnce => 1,
        rumqttc::QoS::ExactlyOnce => 2,
    }
}

/// MQTT 5.0 QoS → wire-level u8 for JSON emission.
fn qos_v5_to_u8(qos: rumqttc::v5::mqttbytes::QoS) -> u8 {
    use rumqttc::v5::mqttbytes::QoS;
    match qos {
        QoS::AtMostOnce => 0,
        QoS::AtLeastOnce => 1,
        QoS::ExactlyOnce => 2,
    }
}

// TLS plumbing for mqtts://.
//
// rumqttc 0.24 pulls in rustls **0.22** (via tokio-rustls 0.25). Recon's direct
// `rustls = "0.23"` dep (used by `tls_probe.rs` / `serve/https.rs`) is a different
// version in the dep graph, so the `Arc<ClientConfig>` stored in
// `rumqttc::TlsConfiguration::Rustls(..)` must be a **0.22** ClientConfig.
//
// We reach that rustls via rumqttc's re-export: `rumqttc::tokio_rustls::rustls`
// (rumqttc → tokio_rustls → rustls). No new direct dep needed for rustls 0.22.
//
// `webpki-roots = "1"` (added as a direct dep, previously transitive via reqwest)
// provides `TrustAnchor<'static>` entries built on `rustls-pki-types = "1"`, which
// **both** rustls 0.22 and 0.23 use — so the trust anchors are portable between
// the two rustls versions.
use rumqttc::tokio_rustls::rustls as mqtt_rustls;

/// Parse the caller's --client-cert / --client-key into a (chain, key)
/// pair typed against rumqttc's rustls 0.22 — the rustls version the
/// broker config needs. Reuses the same PEM-only policy as the HTTPS
/// client-cert path (`src/client_cert.rs`): combined PEMs work,
/// separate cert+key PEM work, encrypted PKCS#8 is refused with an
/// openssl recipe, DER formats are refused.
///
/// Returns `Ok(None)` when no client-cert flags are set.
fn build_client_auth_material(
    args: &crate::cli::Args,
) -> Result<
    Option<(
        Vec<mqtt_rustls::pki_types::CertificateDer<'static>>,
        mqtt_rustls::pki_types::PrivateKeyDer<'static>,
    )>,
> {
    let cert_path = match args.client_cert.as_ref() {
        Some(p) => p,
        None => return Ok(None),
    };
    // Reuse the same format validation as the HTTPS path.
    if !args.cert_type.eq_ignore_ascii_case("PEM") {
        anyhow::bail!(
            "MQTT mTLS: --cert-type {} is not supported under rustls; pass PEM",
            args.cert_type
        );
    }
    if !args.key_type.eq_ignore_ascii_case("PEM") {
        anyhow::bail!(
            "MQTT mTLS: --key-type {} is not supported under rustls; pass PEM",
            args.key_type
        );
    }

    let cert_bytes = std::fs::read(cert_path)
        .with_context(|| format!("--client-cert: read {}", cert_path.display()))?;
    let key_bytes = match args.client_key.as_ref() {
        Some(p) => std::fs::read(p)
            .with_context(|| format!("--client-key: read {}", p.display()))?,
        None => cert_bytes.clone(),
    };

    // Refuse encrypted keys — same stance as src/client_cert.rs.
    if let Ok(s) = std::str::from_utf8(&key_bytes) {
        if s.contains("ENCRYPTED PRIVATE KEY") {
            anyhow::bail!(
                "MQTT mTLS: encrypted PKCS#8 keys not supported. \
                 Decrypt externally (`openssl pkcs8 -in key.enc -out key.pem`) first."
            );
        }
    }

    // Parse cert chain (one or more CERTIFICATE blocks) + first key.
    let cert_pems =
        pem::parse_many(&cert_bytes).context("--client-cert: not valid PEM")?;
    let key_pems =
        pem::parse_many(&key_bytes).context("--client-key: not valid PEM")?;

    let chain: Vec<mqtt_rustls::pki_types::CertificateDer<'static>> = cert_pems
        .iter()
        .filter(|b| b.tag() == "CERTIFICATE")
        .map(|b| mqtt_rustls::pki_types::CertificateDer::from(b.contents().to_vec()))
        .collect();
    if chain.is_empty() {
        anyhow::bail!("--client-cert: no CERTIFICATE blocks in {}", cert_path.display());
    }

    // Key: accept PRIVATE KEY (PKCS#8), RSA PRIVATE KEY (PKCS#1),
    // or EC PRIVATE KEY (SEC1). Fall back to whichever arrives first.
    let key = key_pems
        .iter()
        .find(|b| {
            matches!(
                b.tag(),
                "PRIVATE KEY" | "RSA PRIVATE KEY" | "EC PRIVATE KEY"
            )
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "--client-key: no PRIVATE KEY / RSA PRIVATE KEY / EC PRIVATE KEY block found"
            )
        })?;
    let der = key.contents().to_vec();
    let key_der: mqtt_rustls::pki_types::PrivateKeyDer<'static> = match key.tag() {
        "RSA PRIVATE KEY" => {
            mqtt_rustls::pki_types::PrivatePkcs1KeyDer::from(der).into()
        }
        "EC PRIVATE KEY" => mqtt_rustls::pki_types::PrivateSec1KeyDer::from(der).into(),
        _ => mqtt_rustls::pki_types::PrivatePkcs8KeyDer::from(der).into(),
    };

    Ok(Some((chain, key_der)))
}

/// Build a rustls (0.22) ClientConfig for MQTT-over-TLS. Trusts the Mozilla root
/// CA set via webpki-roots. When `insecure` is true, attaches a verifier that
/// accepts every server certificate — matches recon's HTTPS `-k` flag.
///
/// Uses `builder_with_provider(ring)` (matching `tls_probe.rs` / `serve/https.rs`)
/// so we don't rely on a process-global rustls `CryptoProvider` having been
/// installed: each ClientConfig carries its own provider.
fn build_rustls_config(
    insecure: bool,
    client_auth: Option<(
        Vec<mqtt_rustls::pki_types::CertificateDer<'static>>,
        mqtt_rustls::pki_types::PrivateKeyDer<'static>,
    )>,
) -> Result<mqtt_rustls::ClientConfig> {
    use std::sync::Arc;

    let provider = Arc::new(mqtt_rustls::crypto::ring::default_provider());

    if insecure {
        let builder = mqtt_rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("mqtt TLS: failed to configure protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification));
        Ok(match client_auth {
            Some((chain, key)) => builder
                .with_client_auth_cert(chain, key)
                .context("mqtt mTLS: with_client_auth_cert")?,
            None => builder.with_no_client_auth(),
        })
    } else {
        let mut roots = mqtt_rustls::RootCertStore::empty();
        // webpki-roots 1.x: TLS_SERVER_ROOTS is &[TrustAnchor<'static>] on
        // rustls-pki-types 1.x — compatible with rustls 0.22's RootCertStore.
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let builder = mqtt_rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("mqtt TLS: failed to configure protocol versions")?
            .with_root_certificates(roots);
        Ok(match client_auth {
            Some((chain, key)) => builder
                .with_client_auth_cert(chain, key)
                .context("mqtt mTLS: with_client_auth_cert")?,
            None => builder.with_no_client_auth(),
        })
    }
}

/// Certificate verifier that accepts everything. Used only when the user
/// passes `-k / --insecure`. Same stance as recon's HTTPS path under `-k`.
///
/// Types are imported via `mqtt_rustls` (rustls 0.22) because `ClientConfig`'s
/// `dangerous().with_custom_certificate_verifier(..)` expects the 0.22 trait.
#[derive(Debug)]
struct NoCertificateVerification;

impl mqtt_rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &mqtt_rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[mqtt_rustls::pki_types::CertificateDer<'_>],
        _server_name: &mqtt_rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: mqtt_rustls::pki_types::UnixTime,
    ) -> std::result::Result<mqtt_rustls::client::danger::ServerCertVerified, mqtt_rustls::Error>
    {
        Ok(mqtt_rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &mqtt_rustls::pki_types::CertificateDer<'_>,
        _dss: &mqtt_rustls::DigitallySignedStruct,
    ) -> std::result::Result<
        mqtt_rustls::client::danger::HandshakeSignatureValid,
        mqtt_rustls::Error,
    > {
        Ok(mqtt_rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &mqtt_rustls::pki_types::CertificateDer<'_>,
        _dss: &mqtt_rustls::DigitallySignedStruct,
    ) -> std::result::Result<
        mqtt_rustls::client::danger::HandshakeSignatureValid,
        mqtt_rustls::Error,
    > {
        Ok(mqtt_rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<mqtt_rustls::SignatureScheme> {
        // Mirror the provider's actual scheme list (including RSA-PSS) rather
        // than hardcoding a subset. Matches the pattern in `tls_probe.rs` and
        // keeps TLS 1.3 handshake compatibility broad under `-k`.
        mqtt_rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn configure_tls_v5(
    options: &mut rumqttc::v5::MqttOptions,
    insecure: bool,
    args: &crate::cli::Args,
) -> Result<()> {
    let client_auth = build_client_auth_material(args)?;
    let config = build_rustls_config(insecure, client_auth)?;
    let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
        std::sync::Arc::new(config),
    ));
    options.set_transport(transport);
    Ok(())
}

fn configure_tls_v3(
    options: &mut rumqttc::MqttOptions,
    insecure: bool,
    args: &crate::cli::Args,
) -> Result<()> {
    let client_auth = build_client_auth_material(args)?;
    let config = build_rustls_config(insecure, client_auth)?;
    let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
        std::sync::Arc::new(config),
    ));
    options.set_transport(transport);
    Ok(())
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

#[cfg(test)]
mod subscribe_tests {
    use super::*;

    #[test]
    fn emits_utf8_plain_without_prefix_when_not_verbose() {
        let mut buf = Vec::new();
        emit_subscribe_text(&mut buf, false, "some/topic", b"hello").unwrap();
        assert_eq!(&buf, b"hello\n");
    }

    #[test]
    fn emits_topic_prefix_when_verbose() {
        let mut buf = Vec::new();
        emit_subscribe_text(&mut buf, true, "some/topic", b"hello").unwrap();
        assert_eq!(&buf, b"some/topic hello\n");
    }

    #[test]
    fn escapes_non_utf8_payload_as_hex() {
        let mut buf = Vec::new();
        // 0xFF is not valid UTF-8 on its own — forces the escape path
        emit_subscribe_text(&mut buf, false, "t", &[0xff, b'a']).unwrap();
        assert_eq!(&buf, b"\\xffa\n");
    }

    #[test]
    fn escapes_control_chars_in_non_utf8_branch() {
        let mut buf = Vec::new();
        // \xff (non-UTF-8) + control char 0x01
        emit_subscribe_text(&mut buf, false, "t", &[0xff, 0x01]).unwrap();
        assert_eq!(&buf, b"\\xff\\x01\n");
    }

    #[test]
    fn json_utf8_payload() {
        let mut buf = Vec::new();
        emit_subscribe_json(&mut buf, "some/topic", 1, false, b"hello").unwrap();
        let s = String::from_utf8(buf).unwrap();
        let v: serde_json::Value = serde_json::from_str(s.trim()).unwrap();
        assert_eq!(v["topic"], "some/topic");
        assert_eq!(v["qos"], 1);
        assert_eq!(v["retain"], false);
        assert_eq!(v["payload"], "hello");
    }

    #[test]
    fn json_binary_payload_wraps_base64() {
        let mut buf = Vec::new();
        // 0xFF is not valid UTF-8 — forces base64 wrap
        emit_subscribe_json(&mut buf, "t", 0, true, &[0xff, 0x01]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let v: serde_json::Value = serde_json::from_str(s.trim()).unwrap();
        assert_eq!(v["retain"], true);
        assert!(v["payload"].is_object(), "payload should be an object for non-UTF-8");
        assert_eq!(v["payload"]["base64"], "/wE=");
    }

    #[test]
    fn json_emits_one_line_per_message() {
        let mut buf = Vec::new();
        emit_subscribe_json(&mut buf, "a", 0, false, b"x").unwrap();
        emit_subscribe_json(&mut buf, "b", 0, false, b"y").unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&buf).unwrap().lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line).expect("each line must be valid JSON");
        }
    }
}
