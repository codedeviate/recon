use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde_json::{Map, Value};
use std::io::{IsTerminal, Read, Write};

use crate::cli::Args;

// ── Input kinds ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InputKind {
    /// A JSON object — treated as payload.
    Json(Map<String, Value>),
    /// Two base64url parts (header.payload) — missing signature.
    PartialToken {
        header: Map<String, Value>,
        payload: Map<String, Value>,
    },
    /// Three base64url parts — complete JWT.
    FullToken { token: String },
    /// Single base64url chunk that decoded to a JSON object.
    PayloadOnly(Map<String, Value>),
}

pub fn parse_input(raw: &str) -> Result<InputKind> {
    let raw = raw.trim();
    let dot_count = raw.chars().filter(|&c| c == '.').count();

    match dot_count {
        2 => Ok(InputKind::FullToken {
            token: raw.to_string(),
        }),
        1 => {
            let mut parts = raw.splitn(2, '.');
            let header_part = parts.next().unwrap();
            let payload_part = parts.next().unwrap();
            let header =
                decode_b64_json(header_part).context("Failed to decode JWT header")?;
            let payload =
                decode_b64_json(payload_part).context("Failed to decode JWT payload")?;
            Ok(InputKind::PartialToken { header, payload })
        }
        0 => {
            // Try JSON parse first
            if raw.starts_with('{') {
                let map: Map<String, Value> = serde_json::from_str(raw)
                    .context("Input looks like JSON but could not be parsed")?;
                return Ok(InputKind::Json(map));
            }
            // Try base64url decode
            if let Ok(map) = decode_b64_json(raw) {
                return Ok(InputKind::PayloadOnly(map));
            }
            // Try JSON parse for non-{ prefixed objects
            if let Ok(val) = serde_json::from_str::<Value>(raw) {
                if let Some(map) = val.as_object() {
                    return Ok(InputKind::Json(map.clone()));
                }
            }
            Err(anyhow!(
                "Could not parse input as JSON, JWT, or base64 payload"
            ))
        }
        _ => Err(anyhow!(
            "Could not parse input as JSON, JWT, or base64 payload"
        )),
    }
}

fn decode_b64_json(part: &str) -> Result<Map<String, Value>> {
    let bytes = URL_SAFE_NO_PAD
        .decode(part)
        .with_context(|| format!("Invalid base64url: {}", &part[..part.len().min(20)]))?;
    let val: Value =
        serde_json::from_slice(&bytes).context("Decoded base64url is not valid JSON")?;
    val.as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Decoded base64url is not a JSON object"))
}

pub fn resolve_input(data: Option<&str>, url: &str) -> Result<String> {
    // 1. -d / --data flag (supports @file prefix)
    if let Some(d) = data {
        if let Some(path) = d.strip_prefix('@') {
            return std::fs::read_to_string(path)
                .with_context(|| format!("File not found: {}", path))
                .map(|s| s.trim().to_string());
        }
        return Ok(d.to_string());
    }

    // 2. Positional URL with no protocol → local file path
    if !url.is_empty() && !url.contains("://") {
        return std::fs::read_to_string(url)
            .with_context(|| format!("File not found: {}", url))
            .map(|s| s.trim().to_string());
    }

    // 3. stdin (only if not a TTY)
    if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
        return Ok(buf.trim().to_string());
    }

    Err(anyhow!(
        "No input provided. Use -d <data>, a file path, or pipe data via stdin"
    ))
}

// ── View ─────────────────────────────────────────────────────────────────────

/// Core view logic, writing to any `Write`. Used by `view()` and tests.
pub fn view_to_writer(token: &str, json_report: bool, out: &mut dyn Write) -> Result<()> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "Input must be a JWT with at least header.payload"
        ));
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .context("Failed to decode JWT header")?;
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .context("Failed to decode JWT payload")?;

    let header: Value =
        serde_json::from_slice(&header_bytes).context("JWT header is not valid JSON")?;
    let payload: Value =
        serde_json::from_slice(&payload_bytes).context("JWT payload is not valid JSON")?;

    if json_report {
        let report = serde_json::json!({ "header": header, "payload": payload });
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        writeln!(out, "--- header ---")?;
        writeln!(out, "{}", serde_json::to_string_pretty(&header)?)?;
        writeln!(out)?;
        writeln!(out, "--- payload ---")?;
        writeln!(out, "{}", serde_json::to_string_pretty(&payload)?)?;
    }
    Ok(())
}

pub fn view(args: &Args) -> Result<()> {
    let raw = resolve_input(args.data.as_deref(), args.target_url())?;
    let kind = parse_input(&raw)?;

    let display_token = match kind {
        InputKind::FullToken { token } => token,
        InputKind::PartialToken { ref header, ref payload } => {
            let h = URL_SAFE_NO_PAD.encode(serde_json::to_string(header)?);
            let p = URL_SAFE_NO_PAD.encode(serde_json::to_string(payload)?);
            format!("{}.{}", h, p)
        }
        InputKind::Json(ref map) | InputKind::PayloadOnly(ref map) => {
            // No header available — synthesise empty header for display
            let p = URL_SAFE_NO_PAD.encode(serde_json::to_string(map)?);
            format!("e30.{}", p) // e30 = base64url({})
        }
    };

    view_to_writer(&display_token, args.jwt_json_report, &mut std::io::stdout())
}

// ── Sign helpers ──────────────────────────────────────────────────────────────

/// Extract the payload map from any InputKind.
pub fn extract_payload(kind: InputKind) -> Result<Map<String, Value>> {
    match kind {
        InputKind::Json(map) | InputKind::PayloadOnly(map) => Ok(map),
        InputKind::PartialToken { payload, .. } => Ok(payload),
        InputKind::FullToken { token } => {
            let parts: Vec<&str> = token.split('.').collect();
            decode_b64_json(parts[1])
        }
    }
}

/// Add `key`→`value` to `map` only if the key is not already present.
pub fn merge_claim_if_absent(map: &mut Map<String, Value>, key: &str, value: Value) {
    map.entry(key.to_string()).or_insert(value);
}

/// Parse an algorithm name (case-insensitive). Only HMAC variants supported.
pub fn parse_algorithm(alg: &str) -> Result<Algorithm> {
    match alg.to_uppercase().as_str() {
        "HS256" => Ok(Algorithm::HS256),
        "HS384" => Ok(Algorithm::HS384),
        "HS512" => Ok(Algorithm::HS512),
        other => Err(anyhow!(
            "Unsupported algorithm '{}'. Valid: HS256, HS384, HS512",
            other
        )),
    }
}

/// Sign a claims map and return the JWT string.
pub fn sign_claims(claims: &Map<String, Value>, secret: &str, alg: &str) -> Result<String> {
    let algorithm = parse_algorithm(alg)?;
    let header = Header::new(algorithm);
    let key = EncodingKey::from_secret(secret.as_bytes());
    let val = Value::Object(claims.clone());
    encode(&header, &val, &key).map_err(|e| anyhow!("Failed to sign token: {}", e))
}

/// Current Unix timestamp in seconds.
fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Parse a timestamp string: `"now"` → current time, else parse as `u64`.
fn parse_ts(s: &str) -> Result<u64> {
    if s == "now" {
        return Ok(now_ts());
    }
    s.parse::<u64>()
        .map_err(|_| anyhow!("Invalid timestamp '{}': expected a Unix timestamp integer", s))
}

/// Read the algorithm from an existing token header, falling back to HS256.
fn alg_from_token(token: &str) -> String {
    decode_header(token)
        .ok()
        .and_then(|h| match h.alg {
            Algorithm::HS256 => Some("HS256"),
            Algorithm::HS384 => Some("HS384"),
            Algorithm::HS512 => Some("HS512"),
            _ => None,
        })
        .unwrap_or("HS256")
        .to_string()
}

pub fn sign(args: &Args) -> Result<()> {
    let secret = args
        .jwt_secret
        .as_deref()
        .ok_or_else(|| anyhow!("--jwt-secret is required for --jwt-sign"))?;

    let raw = resolve_input(args.data.as_deref(), args.target_url())?;
    let kind = parse_input(&raw)?;

    // Determine algorithm: --jwt-alg > existing header > HS256
    let alg_str = if let Some(a) = args.jwt_alg.as_deref() {
        a.to_string()
    } else {
        match &kind {
            InputKind::PartialToken { header, .. } => header
                .get("alg")
                .and_then(|v| v.as_str())
                .unwrap_or("HS256")
                .to_string(),
            InputKind::FullToken { token } => alg_from_token(token),
            _ => "HS256".to_string(),
        }
    };

    let mut claims = extract_payload(kind)?;

    // Inject claim flags (only if not already present in payload)
    if let Some(v) = &args.jwt_iss { merge_claim_if_absent(&mut claims, "iss", Value::String(v.clone())); }
    if let Some(v) = &args.jwt_sub { merge_claim_if_absent(&mut claims, "sub", Value::String(v.clone())); }
    if let Some(v) = &args.jwt_aud { merge_claim_if_absent(&mut claims, "aud", Value::String(v.clone())); }
    if let Some(v) = &args.jwt_jti { merge_claim_if_absent(&mut claims, "jti", Value::String(v.clone())); }
    if let Some(v) = &args.jwt_exp {
        let ts = parse_ts(v)?;
        merge_claim_if_absent(&mut claims, "exp", Value::Number(ts.into()));
    }
    if let Some(v) = &args.jwt_nbf {
        let ts = parse_ts(v)?;
        merge_claim_if_absent(&mut claims, "nbf", Value::Number(ts.into()));
    }

    // iat: always add if missing (use --jwt-iat value or current time)
    let iat_ts = match &args.jwt_iat {
        Some(v) => parse_ts(v)?,
        None => now_ts(),
    };
    merge_claim_if_absent(&mut claims, "iat", Value::Number(iat_ts.into()));

    let token = sign_claims(&claims, secret, &alg_str)?;
    println!("{}", token);
    Ok(())
}

// ── Stub public functions (filled in later tasks) ────────────────────────────

pub fn validate(_args: &Args) -> Result<()> { todo!() }

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // base64url of {"alg":"HS256","typ":"JWT"}
    const HEADER_B64: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    // base64url of {"sub":"test"}
    const PAYLOAD_B64: &str = "eyJzdWIiOiJ0ZXN0In0";

    #[test]
    fn parse_json_object() {
        let kind = parse_input(r#"{"sub":"test","iat":1000000000}"#).unwrap();
        assert!(matches!(kind, InputKind::Json(_)));
        if let InputKind::Json(map) = kind {
            assert_eq!(map["sub"], Value::String("test".into()));
        }
    }

    #[test]
    fn parse_full_token() {
        let token = format!("{}.{}.fakesig", HEADER_B64, PAYLOAD_B64);
        let kind = parse_input(&token).unwrap();
        assert!(matches!(kind, InputKind::FullToken { .. }));
        if let InputKind::FullToken { token: t } = kind {
            assert_eq!(t, token);
        }
    }

    #[test]
    fn parse_partial_token() {
        let partial = format!("{}.{}", HEADER_B64, PAYLOAD_B64);
        let kind = parse_input(&partial).unwrap();
        assert!(matches!(kind, InputKind::PartialToken { .. }));
        if let InputKind::PartialToken { header, payload } = kind {
            assert_eq!(header["alg"], Value::String("HS256".into()));
            assert_eq!(payload["sub"], Value::String("test".into()));
        }
    }

    #[test]
    fn parse_payload_only_b64() {
        let encoded = URL_SAFE_NO_PAD.encode(r#"{"sub":"test"}"#);
        let kind = parse_input(&encoded).unwrap();
        assert!(matches!(kind, InputKind::PayloadOnly(_)));
        if let InputKind::PayloadOnly(map) = kind {
            assert_eq!(map["sub"], Value::String("test".into()));
        }
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(parse_input("not-valid!!!").is_err());
    }

    mod view_tests {
        use super::*;

        fn make_token(claims: &Value) -> String {
            let key = EncodingKey::from_secret(b"secret");
            encode(&Header::new(Algorithm::HS256), claims, &key).unwrap()
        }

        #[test]
        fn view_labeled_sections() {
            let token = make_token(&serde_json::json!({"sub": "alice"}));
            let mut out = Vec::<u8>::new();
            view_to_writer(&token, false, &mut out).unwrap();
            let s = String::from_utf8(out).unwrap();
            assert!(s.contains("--- header ---"), "missing header section");
            assert!(s.contains("--- payload ---"), "missing payload section");
            assert!(s.contains("alice"), "payload content missing");
        }

        #[test]
        fn view_json_report() {
            let token = make_token(&serde_json::json!({"sub": "alice"}));
            let mut out = Vec::<u8>::new();
            view_to_writer(&token, true, &mut out).unwrap();
            let s = String::from_utf8(out).unwrap();
            let parsed: Value = serde_json::from_str(&s).unwrap();
            assert!(parsed["header"].is_object());
            assert_eq!(parsed["payload"]["sub"], Value::String("alice".into()));
        }

        #[test]
        fn view_partial_token_shows_available_parts() {
            let partial = format!("{}.{}", HEADER_B64, PAYLOAD_B64);
            let kind = parse_input(&partial).unwrap();
            // Reconstruct a display string and pass to view_to_writer
            if let InputKind::PartialToken { header, payload } = kind {
                let h = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
                let p = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
                let display = format!("{}.{}", h, p);
                let mut out = Vec::<u8>::new();
                view_to_writer(&display, false, &mut out).unwrap();
                let s = String::from_utf8(out).unwrap();
                assert!(s.contains("--- header ---"));
                assert!(s.contains("--- payload ---"));
            } else {
                panic!("Expected PartialToken");
            }
        }
    }

    mod sign_tests {
        use super::*;

        #[test]
        fn sign_json_payload_produces_verifiable_token() {
            let kind = parse_input(r#"{"sub":"alice"}"#).unwrap();
            let mut claims = extract_payload(kind).unwrap();
            merge_claim_if_absent(&mut claims, "iat", serde_json::json!(1000000000_u64));

            let token = sign_claims(&claims, "secret", "HS256").unwrap();

            // Round-trip: verify with jsonwebtoken
            let mut v = Validation::new(Algorithm::HS256);
            v.validate_exp = false;
            v.required_spec_claims = std::collections::HashSet::new();
            let key = DecodingKey::from_secret(b"secret");
            let data = decode::<Value>(&token, &key, &v).unwrap();
            assert_eq!(data.claims["sub"], Value::String("alice".into()));
        }

        #[test]
        fn sign_partial_token_preserves_header_alg() {
            let header_json = r#"{"alg":"HS384","typ":"JWT"}"#;
            let payload_json = r#"{"sub":"bob"}"#;
            let h = URL_SAFE_NO_PAD.encode(header_json.as_bytes());
            let p = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
            let partial = format!("{}.{}", h, p);

            let kind = parse_input(&partial).unwrap();
            let alg_str = match &kind {
                InputKind::PartialToken { header, .. } => {
                    header.get("alg").and_then(|v| v.as_str()).unwrap_or("HS256").to_string()
                }
                _ => "HS256".to_string(),
            };
            let claims = extract_payload(kind).unwrap();
            let token = sign_claims(&claims, "secret", &alg_str).unwrap();

            let mut v = Validation::new(Algorithm::HS384);
            v.validate_exp = false;
            v.required_spec_claims = std::collections::HashSet::new();
            let key = DecodingKey::from_secret(b"secret");
            let data = decode::<Value>(&token, &key, &v).unwrap();
            assert_eq!(data.claims["sub"], Value::String("bob".into()));
        }

        #[test]
        fn merge_claim_does_not_overwrite_existing() {
            let kind = parse_input(r#"{"iss":"original"}"#).unwrap();
            let mut claims = extract_payload(kind).unwrap();
            merge_claim_if_absent(&mut claims, "iss", serde_json::json!("injected"));
            assert_eq!(claims["iss"], Value::String("original".into()));
        }

        #[test]
        fn parse_algorithm_rejects_unknown() {
            assert!(parse_algorithm("RS256").is_err());
            assert!(parse_algorithm("XYZ").is_err());
        }

        #[test]
        fn parse_algorithm_accepts_hs_variants() {
            assert!(parse_algorithm("HS256").is_ok());
            assert!(parse_algorithm("HS384").is_ok());
            assert!(parse_algorithm("HS512").is_ok());
            assert!(parse_algorithm("hs256").is_ok()); // case-insensitive
        }
    }
}
