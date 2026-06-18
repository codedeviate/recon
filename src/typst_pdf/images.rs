//! Markdown image source resolution for the typst PDF engine.
//!
//! Markdown `![alt](src)` images can reference three kinds of source:
//!
//! - **Local** filesystem paths, resolved relative to the markdown file's
//!   directory (or the current working directory for stdin).
//! - **`data:` URIs** carrying base64-encoded bytes inline.
//! - **Remote** `http(s)` URLs, fetched over the network.
//!
//! [`resolve`] turns any of these into raw image bytes plus an optional format
//! hint (`png`, `jpg`, `gif`, `svg`). The translator embeds those bytes inline
//! in the generated typst source via `#image(bytes((...)))` — a detached main
//! source cannot resolve filesystem paths, so inline bytes are the only
//! workable embedding mechanism.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Classification of a markdown image `src` attribute.
#[derive(Debug, PartialEq, Eq)]
pub enum ImgSrc {
    /// A local filesystem path (resolved relative to the markdown base dir).
    Local(PathBuf),
    /// A `data:` URI with a decoded MIME type and raw bytes.
    Data { mime: String, bytes: Vec<u8> },
    /// A remote `http(s)` URL.
    Remote(String),
}

/// Classify a markdown image source string into one of the three [`ImgSrc`]
/// variants. `data:` URIs are decoded eagerly (base64 only); a malformed
/// data URI classifies as `Data` with empty bytes so the caller surfaces a
/// resolve error rather than treating it as a local path.
pub fn classify(src: &str) -> ImgSrc {
    let lower = src.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return ImgSrc::Remote(src.trim().to_string());
    }
    if lower.starts_with("data:") {
        // data:[<mime>][;base64],<data>
        let rest = &src.trim()[5..];
        let (meta, payload) = match rest.split_once(',') {
            Some((m, p)) => (m, p),
            None => ("", ""),
        };
        let is_base64 = meta.to_ascii_lowercase().contains(";base64");
        let mime = meta.split(';').next().unwrap_or("").to_string();
        let bytes = if is_base64 {
            STANDARD.decode(payload.trim()).unwrap_or_default()
        } else {
            // Non-base64 data URIs are percent/plain text — treat the payload
            // as raw UTF-8 bytes (rare for images; best-effort).
            payload.as_bytes().to_vec()
        };
        return ImgSrc::Data { mime, bytes };
    }
    // Strip a leading file:// scheme if present, else treat as a path.
    let path = src.trim().strip_prefix("file://").unwrap_or(src.trim());
    ImgSrc::Local(PathBuf::from(path))
}

/// Map a MIME subtype or file extension to a typst image format hint.
fn format_hint_from_mime(mime: &str) -> Option<String> {
    let sub = mime.rsplit('/').next().unwrap_or("");
    match sub.to_ascii_lowercase().as_str() {
        "png" => Some("png".to_string()),
        "jpeg" | "jpg" => Some("jpg".to_string()),
        "gif" => Some("gif".to_string()),
        "svg" | "svg+xml" => Some("svg".to_string()),
        _ => None,
    }
}

/// Map a file extension to a typst image format hint.
fn format_hint_from_ext(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("png".to_string()),
        "jpeg" | "jpg" => Some("jpg".to_string()),
        "gif" => Some("gif".to_string()),
        "svg" => Some("svg".to_string()),
        _ => None,
    }
}

/// Resolve a markdown image source to raw bytes plus an optional typst format
/// hint. Local paths are read relative to `base_dir`; `data:` URIs are decoded;
/// remote URLs are fetched with `http`.
///
/// Remote and local errors are returned as `Err` so the caller can fall back to
/// rendering the alt text (a missing image must never abort the whole PDF).
pub fn resolve(
    src: &str,
    base_dir: &Path,
    http: &reqwest::blocking::Client,
) -> Result<(Vec<u8>, Option<String>)> {
    match classify(src) {
        ImgSrc::Local(rel) => {
            let path = if rel.is_absolute() {
                rel.clone()
            } else {
                base_dir.join(&rel)
            };
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read local image '{}'", path.display()))?;
            Ok((bytes, format_hint_from_ext(&rel)))
        }
        ImgSrc::Data { mime, bytes } => {
            if bytes.is_empty() {
                return Err(anyhow!("data URI decoded to zero bytes"));
            }
            Ok((bytes, format_hint_from_mime(&mime)))
        }
        ImgSrc::Remote(url) => {
            let resp = http
                .get(&url)
                .send()
                .with_context(|| format!("fetch remote image '{url}'"))?;
            if !resp.status().is_success() {
                return Err(anyhow!("remote image '{url}' returned HTTP {}", resp.status()));
            }
            let hint = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .and_then(format_hint_from_mime);
            let hint = hint.or_else(|| {
                // Fall back to the URL path extension.
                let path = url.split(['?', '#']).next().unwrap_or(&url);
                format_hint_from_ext(Path::new(path))
            });
            let bytes = resp.bytes().context("read remote image body")?.to_vec();
            Ok((bytes, hint))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_remote() {
        assert_eq!(
            classify("https://example.com/a.png"),
            ImgSrc::Remote("https://example.com/a.png".to_string())
        );
        assert_eq!(
            classify("http://example.com/a.png"),
            ImgSrc::Remote("http://example.com/a.png".to_string())
        );
    }

    #[test]
    fn classify_local() {
        assert_eq!(classify("img/logo.png"), ImgSrc::Local(PathBuf::from("img/logo.png")));
        assert_eq!(classify("/abs/x.jpg"), ImgSrc::Local(PathBuf::from("/abs/x.jpg")));
    }

    #[test]
    fn classify_data_base64() {
        // A tiny valid 1x1 transparent PNG.
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let uri = format!("data:image/png;base64,{b64}");
        match classify(&uri) {
            ImgSrc::Data { mime, bytes } => {
                assert_eq!(mime, "image/png");
                // PNG magic header: 0x89 'P' 'N' 'G'.
                assert_eq!(&bytes[..4], &[0x89, 0x50, 0x4E, 0x47]);
            }
            other => panic!("expected Data, got {other:?}"),
        }
    }

    #[test]
    fn data_decode_known_png_header() {
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let raw = STANDARD.decode(b64).unwrap();
        assert!(raw.len() > 8);
        assert_eq!(&raw[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn format_hints() {
        assert_eq!(format_hint_from_mime("image/png").as_deref(), Some("png"));
        assert_eq!(format_hint_from_mime("image/jpeg").as_deref(), Some("jpg"));
        assert_eq!(format_hint_from_mime("image/svg+xml").as_deref(), Some("svg"));
        assert_eq!(format_hint_from_ext(Path::new("a/b.GIF")).as_deref(), Some("gif"));
        assert_eq!(format_hint_from_ext(Path::new("noext")), None);
    }
}
