//! `--export-pdf-page` — render a single PDF page to a raster image
//! through `agent-browser` (Chrome). See the design spec for the full
//! flow; this module owns the option parsing, format dispatch, and the
//! agent-browser drive sequence.

use anyhow::{anyhow, Context, Result};

/// Parse `"WxH"` (e.g. `"1024x1366"`) into `(w, h)`. Both must be > 0.
pub fn parse_viewport(s: &str) -> Result<(u32, u32)> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| anyhow!("--pdf-viewport must be WxH (e.g. 1024x1366)"))?;
    let w: u32 = w
        .trim()
        .parse()
        .map_err(|_| anyhow!("--pdf-viewport width '{w}' is not a positive integer"))?;
    let h: u32 = h
        .trim()
        .parse()
        .map_err(|_| anyhow!("--pdf-viewport height '{h}' is not a positive integer"))?;
    if w == 0 || h == 0 {
        return Err(anyhow!("--pdf-viewport: width and height must be > 0"));
    }
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_viewport_ok() {
        assert_eq!(parse_viewport("1024x1366").unwrap(), (1024, 1366));
        assert_eq!(parse_viewport("1x1").unwrap(), (1, 1));
    }

    #[test]
    fn parse_viewport_rejects_missing_separator() {
        let e = parse_viewport("1024").unwrap_err().to_string();
        assert!(e.contains("WxH"), "got: {e}");
    }

    #[test]
    fn parse_viewport_rejects_non_numeric() {
        assert!(parse_viewport("Ax10").is_err());
        assert!(parse_viewport("10xB").is_err());
    }

    #[test]
    fn parse_viewport_rejects_zero() {
        let e = parse_viewport("0x100").unwrap_err().to_string();
        assert!(e.contains("> 0"), "got: {e}");
    }
}
