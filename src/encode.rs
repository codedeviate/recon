//! `--encode <FORMAT>`: generate QR codes, DataMatrix, or 1D barcodes from
//! the positional text (or stdin, or --from-file). Renders to ASCII, SVG,
//! or PNG. PNG uses the `png` crate directly for a small dependency tree.

use anyhow::{anyhow, Result};
use std::path::Path;

/// Which code standard to encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Qr,
    DataMatrix,
    Code128,
    Code39,
    Ean13,
    UpcA,
}

impl Format {
    pub fn canonical(&self) -> &'static str {
        match self {
            Format::Qr => "qr",
            Format::DataMatrix => "datamatrix",
            Format::Code128 => "code128",
            Format::Code39 => "code39",
            Format::Ean13 => "ean13",
            Format::UpcA => "upca",
        }
    }

    pub fn kind(&self) -> MatrixKind {
        match self {
            Format::Qr | Format::DataMatrix => MatrixKind::TwoD,
            Format::Code128 | Format::Code39 | Format::Ean13 | Format::UpcA => MatrixKind::OneD,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Format::Qr => "any text",
            Format::DataMatrix => "any bytes",
            Format::Code128 => "ASCII",
            Format::Code39 => "uppercase alphanumeric + -.$/+%* ",
            Format::Ean13 => "12 or 13 digits",
            Format::UpcA => "11 or 12 digits",
        }
    }

    pub const ALL: &'static [Format] = &[
        Format::Qr,
        Format::DataMatrix,
        Format::Code128,
        Format::Code39,
        Format::Ean13,
        Format::UpcA,
    ];
}

/// Rendering kind — drives how the renderers lay out the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixKind {
    TwoD,
    OneD,
}

/// Output rendering format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Ascii,
    Svg,
    Png,
}

/// Parse a user-supplied encode-format name. Case-insensitive.
pub fn parse_format(input: &str) -> Result<Format> {
    let lower = input.trim().to_ascii_lowercase();
    for f in Format::ALL {
        if f.canonical() == lower {
            return Ok(*f);
        }
    }
    let supported: Vec<&str> = Format::ALL.iter().map(|f| f.canonical()).collect();
    Err(anyhow!(
        "unknown encode format '{input}'; supported: {}",
        supported.join(", "),
    ))
}

/// Parse `--encode-format` into `OutputFormat`. Case-insensitive.
pub fn parse_output_format(input: &str) -> Result<OutputFormat> {
    match input.trim().to_ascii_lowercase().as_str() {
        "ascii" => Ok(OutputFormat::Ascii),
        "svg" => Ok(OutputFormat::Svg),
        "png" => Ok(OutputFormat::Png),
        _ => Err(anyhow!(
            "unknown encode-format '{input}'; supported: ascii, svg, png"
        )),
    }
}

/// Resolve the output format given explicit override, output file path.
pub fn resolve_output_format(
    explicit: Option<&str>,
    output_path: Option<&Path>,
) -> Result<OutputFormat> {
    if let Some(s) = explicit {
        return parse_output_format(s);
    }
    if let Some(path) = output_path {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_ascii_lowercase().as_str() {
                "svg" => return Ok(OutputFormat::Svg),
                "png" => return Ok(OutputFormat::Png),
                _ => {}
            }
        }
    }
    Ok(OutputFormat::Ascii)
}

/// Bit matrix produced by an encoder. Renderers consume this.
#[derive(Debug, Clone)]
pub struct BitMatrix {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<bool>,
    pub kind: MatrixKind,
}

impl BitMatrix {
    pub fn get(&self, x: u32, y: u32) -> bool {
        self.bits[(y * self.width + x) as usize]
    }
}

// ---- Encoders — Task 3 -------------------------------------------------

pub fn encode(format: Format, input: &[u8]) -> Result<BitMatrix> {
    match format {
        Format::Qr => encode_qr(input),
        Format::DataMatrix => encode_datamatrix(input),
        Format::Code128 => encode_1d(format, input),
        Format::Code39 => encode_1d(format, input),
        Format::Ean13 => encode_1d(format, input),
        Format::UpcA => encode_1d(format, input),
    }
}

fn encode_qr(input: &[u8]) -> Result<BitMatrix> {
    let qr = qrcode::QrCode::new(input)
        .map_err(|e| anyhow!("qr encode error: {e}"))?;
    let width = qr.width() as u32;
    let bits: Vec<bool> = qr
        .to_colors()
        .into_iter()
        .map(|c| c == qrcode::Color::Dark)
        .collect();
    Ok(BitMatrix {
        width,
        height: width,
        bits,
        kind: MatrixKind::TwoD,
    })
}

fn encode_datamatrix(input: &[u8]) -> Result<BitMatrix> {
    let dm = datamatrix::DataMatrix::encode(input, datamatrix::SymbolList::default())
        .map_err(|e| anyhow!("datamatrix encode error: {e:?}"))?;
    let bitmap = dm.bitmap();
    let width = bitmap.width() as u32;
    let height = bitmap.height() as u32;
    let bits: Vec<bool> = bitmap.bits().to_vec();
    Ok(BitMatrix {
        width,
        height,
        bits,
        kind: MatrixKind::TwoD,
    })
}

fn encode_1d(format: Format, input: &[u8]) -> Result<BitMatrix> {
    let text = std::str::from_utf8(input)
        .map_err(|_| anyhow!("{}: input must be valid UTF-8", format.canonical()))?
        .trim();

    let bars: Vec<u8> = match format {
        Format::Code128 => {
            let bc = barcoders::sym::code128::Code128::new(text)
                .map_err(|e| anyhow!("code128: {e}"))?;
            bc.encode()
        }
        Format::Code39 => {
            let bc = barcoders::sym::code39::Code39::new(text)
                .map_err(|e| anyhow!("code39: {e}"))?;
            bc.encode()
        }
        Format::Ean13 => {
            let bc = barcoders::sym::ean13::EAN13::new(text)
                .map_err(|e| anyhow!("ean13: {e}"))?;
            bc.encode()
        }
        Format::UpcA => {
            let bc = barcoders::sym::ean13::UPCA::new(text)
                .map_err(|e| anyhow!("upca: {e}"))?;
            bc.encode()
        }
        _ => unreachable!("encode_1d called with non-1D format"),
    };

    let width = bars.len() as u32;
    if width == 0 {
        return Err(anyhow!("{}: encoded pattern is empty", format.canonical()));
    }
    let bits: Vec<bool> = bars.iter().map(|b| *b == 1).collect();
    Ok(BitMatrix {
        width,
        height: 1,
        bits,
        kind: MatrixKind::OneD,
    })
}

// ---- Renderer stubs — filled in by Task 4 ------------------------------

pub fn render_ascii(_matrix: &BitMatrix) -> String {
    String::new()
}

pub fn render_svg(_matrix: &BitMatrix) -> String {
    String::new()
}

pub fn render_png(_matrix: &BitMatrix) -> Result<Vec<u8>> {
    Err(anyhow!("encode::render_png not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_format_all_names() {
        assert_eq!(parse_format("qr").unwrap(), Format::Qr);
        assert_eq!(parse_format("datamatrix").unwrap(), Format::DataMatrix);
        assert_eq!(parse_format("code128").unwrap(), Format::Code128);
        assert_eq!(parse_format("code39").unwrap(), Format::Code39);
        assert_eq!(parse_format("ean13").unwrap(), Format::Ean13);
        assert_eq!(parse_format("upca").unwrap(), Format::UpcA);
    }

    #[test]
    fn parse_format_case_insensitive() {
        assert_eq!(parse_format("QR").unwrap(), Format::Qr);
        assert_eq!(parse_format("DataMatrix").unwrap(), Format::DataMatrix);
        assert_eq!(parse_format("CODE128").unwrap(), Format::Code128);
    }

    #[test]
    fn parse_format_unknown_lists_supported() {
        let err = parse_format("aztec").unwrap_err().to_string();
        assert!(err.contains("aztec"), "got: {err}");
        assert!(err.contains("qr"), "got: {err}");
        assert!(err.contains("upca"), "got: {err}");
    }

    #[test]
    fn parse_output_format_happy() {
        assert_eq!(parse_output_format("ascii").unwrap(), OutputFormat::Ascii);
        assert_eq!(parse_output_format("SVG").unwrap(), OutputFormat::Svg);
        assert_eq!(parse_output_format("Png").unwrap(), OutputFormat::Png);
    }

    #[test]
    fn parse_output_format_unknown() {
        let err = parse_output_format("jpeg").unwrap_err().to_string();
        assert!(err.contains("jpeg"), "got: {err}");
        assert!(err.contains("ascii"), "got: {err}");
    }

    #[test]
    fn resolve_output_format_explicit_wins_over_extension() {
        let path = PathBuf::from("foo.svg");
        let got = resolve_output_format(Some("png"), Some(&path)).unwrap();
        assert_eq!(got, OutputFormat::Png);
    }

    #[test]
    fn resolve_output_format_extension_svg() {
        let path = PathBuf::from("foo.svg");
        assert_eq!(
            resolve_output_format(None, Some(&path)).unwrap(),
            OutputFormat::Svg,
        );
    }

    #[test]
    fn resolve_output_format_extension_png_case_insensitive() {
        let path = PathBuf::from("OUT.PNG");
        assert_eq!(
            resolve_output_format(None, Some(&path)).unwrap(),
            OutputFormat::Png,
        );
    }

    #[test]
    fn resolve_output_format_unknown_extension_is_ascii() {
        let path = PathBuf::from("foo.bin");
        assert_eq!(
            resolve_output_format(None, Some(&path)).unwrap(),
            OutputFormat::Ascii,
        );
    }

    #[test]
    fn resolve_output_format_no_path_is_ascii() {
        assert_eq!(
            resolve_output_format(None, None).unwrap(),
            OutputFormat::Ascii,
        );
    }

    #[test]
    fn format_kind_grouping() {
        assert_eq!(Format::Qr.kind(), MatrixKind::TwoD);
        assert_eq!(Format::DataMatrix.kind(), MatrixKind::TwoD);
        assert_eq!(Format::Code128.kind(), MatrixKind::OneD);
        assert_eq!(Format::Code39.kind(), MatrixKind::OneD);
        assert_eq!(Format::Ean13.kind(), MatrixKind::OneD);
        assert_eq!(Format::UpcA.kind(), MatrixKind::OneD);
    }

    #[test]
    fn format_all_has_six_variants() {
        assert_eq!(Format::ALL.len(), 6);
    }

    #[test]
    fn encode_qr_produces_square_matrix() {
        let m = encode(Format::Qr, b"hello recon").unwrap();
        assert!(m.width > 0);
        assert_eq!(m.width, m.height);
        assert_eq!(m.bits.len(), (m.width * m.height) as usize);
        assert_eq!(m.kind, MatrixKind::TwoD);
    }

    #[test]
    fn encode_qr_accepts_utf8() {
        let m = encode(Format::Qr, "héllo 🙂".as_bytes()).unwrap();
        assert!(m.width > 0);
    }

    #[test]
    fn encode_datamatrix_produces_some_matrix() {
        let m = encode(Format::DataMatrix, b"199001011234").unwrap();
        assert!(m.width > 0);
        assert!(m.height > 0);
        assert_eq!(m.kind, MatrixKind::TwoD);
    }

    #[test]
    fn encode_code128_produces_1d_matrix() {
        // Code128 requires a charset prefix: À (0xC0) = charset A
        let m = encode(Format::Code128, "ÀRECON-TEST-001".as_bytes()).unwrap();
        assert!(m.width > 0);
        assert_eq!(m.height, 1);
        assert_eq!(m.kind, MatrixKind::OneD);
    }

    #[test]
    fn encode_code39_rejects_lowercase() {
        let err = encode(Format::Code39, b"lowercase").unwrap_err().to_string();
        assert!(err.contains("code39"), "got: {err}");
    }

    #[test]
    fn encode_code39_accepts_uppercase() {
        let m = encode(Format::Code39, b"HELLO-42").unwrap();
        assert_eq!(m.kind, MatrixKind::OneD);
    }

    #[test]
    fn encode_ean13_12_digits_ok() {
        let m = encode(Format::Ean13, b"590123412345").unwrap();
        assert_eq!(m.kind, MatrixKind::OneD);
    }

    #[test]
    fn encode_ean13_rejects_wrong_length() {
        let err = encode(Format::Ean13, b"1234").unwrap_err().to_string();
        assert!(err.contains("ean13"), "got: {err}");
    }

    #[test]
    fn encode_ean13_rejects_non_digits() {
        let err = encode(Format::Ean13, b"59012341234x").unwrap_err().to_string();
        assert!(err.contains("ean13"), "got: {err}");
    }

    #[test]
    fn encode_upca_12_digits_ok() {
        // UPCA is an alias for EAN13 in barcoders; requires 12 digits (11 + check digit)
        let m = encode(Format::UpcA, b"012345678905").unwrap();
        assert_eq!(m.kind, MatrixKind::OneD);
    }
}
