//! `--decode <IMAGE>` — barcode / QR / data-matrix image decoding.
//!
//! Wraps `rxing::helpers::detect_in_file` with a curl-friendly CLI
//! shell: reads an image (PNG/JPEG/WebP/…) from a path or stdin,
//! attempts to decode, and prints the text + detected format. An
//! optional `--decode-hints` list restricts the scan to specific
//! formats (faster; avoids ambiguity with codes that share prefixes).

use anyhow::{anyhow, bail, Context, Result};
use rxing::BarcodeFormat;

use crate::cli::Args;

/// Parse a single `--decode-hints` token into a `BarcodeFormat`.
/// Accepts curl-compatible lowercase (`qr`, `ean13`, `code128`) plus
/// the full `QR_CODE` form rxing uses internally.
fn parse_format(s: &str) -> Result<BarcodeFormat> {
    match s.trim().to_ascii_lowercase().as_str() {
        "qr" | "qr_code" | "qrcode" => Ok(BarcodeFormat::QR_CODE),
        "datamatrix" | "data_matrix" | "dm" => Ok(BarcodeFormat::DATA_MATRIX),
        "aztec" => Ok(BarcodeFormat::AZTEC),
        "pdf417" | "pdf_417" => Ok(BarcodeFormat::PDF_417),
        "maxicode" => Ok(BarcodeFormat::MAXICODE),
        "code128" | "code_128" => Ok(BarcodeFormat::CODE_128),
        "code39" | "code_39" => Ok(BarcodeFormat::CODE_39),
        "code93" | "code_93" => Ok(BarcodeFormat::CODE_93),
        "codabar" => Ok(BarcodeFormat::CODABAR),
        "ean13" | "ean_13" => Ok(BarcodeFormat::EAN_13),
        "ean8" | "ean_8" => Ok(BarcodeFormat::EAN_8),
        "itf" => Ok(BarcodeFormat::ITF),
        "upca" | "upc_a" => Ok(BarcodeFormat::UPC_A),
        "upce" | "upc_e" => Ok(BarcodeFormat::UPC_E),
        "rss14" | "rss_14" => Ok(BarcodeFormat::RSS_14),
        "rss_expanded" | "rssexpanded" => Ok(BarcodeFormat::RSS_EXPANDED),
        other => bail!("unknown --decode-hints format '{other}' (use `recon --help decode` for the full list)"),
    }
}

/// Canonical lowercase name for a rxing `BarcodeFormat`. Stable across
/// releases so scripts can switch on it.
pub(crate) fn format_name(fmt: &BarcodeFormat) -> &'static str {
    match fmt {
        BarcodeFormat::QR_CODE => "qr",
        BarcodeFormat::DATA_MATRIX => "datamatrix",
        BarcodeFormat::AZTEC => "aztec",
        BarcodeFormat::PDF_417 => "pdf417",
        BarcodeFormat::MAXICODE => "maxicode",
        BarcodeFormat::CODE_128 => "code128",
        BarcodeFormat::CODE_39 => "code39",
        BarcodeFormat::CODE_93 => "code93",
        BarcodeFormat::CODABAR => "codabar",
        BarcodeFormat::EAN_13 => "ean13",
        BarcodeFormat::EAN_8 => "ean8",
        BarcodeFormat::ITF => "itf",
        BarcodeFormat::UPC_A => "upca",
        BarcodeFormat::UPC_E => "upce",
        BarcodeFormat::RSS_14 => "rss14",
        BarcodeFormat::RSS_EXPANDED => "rss_expanded",
        BarcodeFormat::MICRO_QR_CODE => "micro_qr",
        BarcodeFormat::RECTANGULAR_MICRO_QR_CODE => "rmqr",
        BarcodeFormat::TELEPEN => "telepen",
        BarcodeFormat::DXFilmEdge => "dxfilmedge",
        BarcodeFormat::UPC_EAN_EXTENSION => "upc_ean_extension",
        BarcodeFormat::UNSUPORTED_FORMAT => "unsupported",
    }
}

/// Result of a successful decode. Separated from rxing's `RXingResult`
/// so callers (CLI + script binding) share one shape.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub text: String,
    pub format: &'static str,
}

/// Decode from a file path. Keeps the path-based flow because rxing's
/// `detect_in_file` opens the image and drops straight into its reader
/// pipeline — cheaper than routing through bytes.
pub fn decode_file(path: &str, hints: &[BarcodeFormat]) -> Result<Decoded> {
    let result = if hints.is_empty() {
        rxing::helpers::detect_in_file(path, None)
    } else if hints.len() == 1 {
        rxing::helpers::detect_in_file(path, Some(hints[0]))
    } else {
        // rxing's detect_in_file takes a single format hint. For
        // multi-format hints, fall back to the untyped path (it already
        // scans all enabled formats) and filter the result.
        rxing::helpers::detect_in_file(path, None)
    }
    .map_err(|e| anyhow!("decode error: {e:?}"))?;

    let fmt = *result.getBarcodeFormat();
    if !hints.is_empty() && !hints.contains(&fmt) {
        bail!(
            "decoded a {} barcode but --decode-hints restricted to {:?}",
            format_name(&fmt),
            hints.iter().map(format_name).collect::<Vec<_>>(),
        );
    }

    Ok(Decoded {
        text: result.getText().to_string(),
        format: format_name(&fmt),
    })
}

/// Decode from an in-memory image blob. Writes to a tempfile and calls
/// `decode_file` — rxing expects filesystem paths for format detection
/// (its `detect_in_luma` requires pre-parsed pixel data and bypasses
/// rxing's own decoder for PNG/JPEG headers).
pub fn decode_bytes(bytes: &[u8], hints: &[BarcodeFormat]) -> Result<Decoded> {
    use std::io::Write;
    let mut tmp = tempfile::Builder::new()
        .prefix("recon-decode-")
        .suffix(".img")
        .tempfile()
        .context("decode: create tempfile")?;
    tmp.write_all(bytes).context("decode: write tempfile")?;
    tmp.flush().ok();
    let path = tmp
        .path()
        .to_str()
        .ok_or_else(|| anyhow!("decode: tempfile path is not UTF-8"))?;
    decode_file(path, hints)
}

/// CLI entry point. Reads the image from the flag value (path or `-`
/// for stdin) and prints one line: `<FORMAT>\t<TEXT>` (or JSON when
/// `--json` is set).
pub fn run(args: &Args) -> Result<()> {
    let src = args
        .decode
        .as_ref()
        .context("--decode requires an image path (or `-` for stdin)")?;

    let hints: Vec<BarcodeFormat> = match args.decode_hints.as_deref() {
        Some(s) if !s.trim().is_empty() => s
            .split(',')
            .map(|token| parse_format(token.trim()))
            .collect::<Result<Vec<_>>>()?,
        _ => Vec::new(),
    };

    let decoded = if src == "-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
            .context("--decode: read stdin")?;
        decode_bytes(&buf, &hints)?
    } else {
        decode_file(src, &hints)?
    };

    println!("{}\t{}", decoded.format, decoded.text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_format_accepts_curl_style() {
        assert!(matches!(
            parse_format("qr").unwrap(),
            BarcodeFormat::QR_CODE
        ));
        assert!(matches!(
            parse_format("QR_CODE").unwrap(),
            BarcodeFormat::QR_CODE
        ));
        assert!(matches!(
            parse_format("pdf417").unwrap(),
            BarcodeFormat::PDF_417
        ));
        assert!(matches!(
            parse_format("aztec").unwrap(),
            BarcodeFormat::AZTEC
        ));
        assert!(matches!(
            parse_format("ean13").unwrap(),
            BarcodeFormat::EAN_13
        ));
        assert!(parse_format("unknown-format").is_err());
    }

    #[test]
    fn format_name_stable_for_common_types() {
        assert_eq!(format_name(&BarcodeFormat::QR_CODE), "qr");
        assert_eq!(format_name(&BarcodeFormat::DATA_MATRIX), "datamatrix");
        assert_eq!(format_name(&BarcodeFormat::AZTEC), "aztec");
        assert_eq!(format_name(&BarcodeFormat::PDF_417), "pdf417");
        assert_eq!(format_name(&BarcodeFormat::CODE_128), "code128");
    }
}
