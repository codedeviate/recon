use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
#[allow(unused_imports)]
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde_json::{Map, Value};
use std::io::{IsTerminal, Read};

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

// ── Stub public functions (filled in later tasks) ────────────────────────────

pub fn view(_args: &Args) -> Result<()> { todo!() }
pub fn sign(_args: &Args) -> Result<()> { todo!() }
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
}
