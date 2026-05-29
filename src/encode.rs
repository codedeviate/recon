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
    Aztec,
    Pdf417,
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
            Format::Aztec => "aztec",
            Format::Pdf417 => "pdf417",
        }
    }

    pub fn kind(&self) -> MatrixKind {
        match self {
            Format::Qr
            | Format::DataMatrix
            | Format::Aztec
            | Format::Pdf417 => MatrixKind::TwoD,
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
            Format::Aztec => "any text (compact; popular on transit tickets)",
            Format::Pdf417 => "any bytes (larger rectangular; used on IDs and shipping)",
        }
    }

    pub const ALL: &'static [Format] = &[
        Format::Qr,
        Format::DataMatrix,
        Format::Code128,
        Format::Code39,
        Format::Ean13,
        Format::UpcA,
        Format::Aztec,
        Format::Pdf417,
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

/// QR error-correction level. Controls how much damage the code can
/// sustain and still decode. Bigger level = more redundancy = larger
/// matrix. Default `M` matches curl/qrcode's traditional pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QrLevel {
    L,
    #[default]
    M,
    Q,
    H,
}

impl QrLevel {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "L" => Ok(QrLevel::L),
            "M" => Ok(QrLevel::M),
            "Q" => Ok(QrLevel::Q),
            "H" => Ok(QrLevel::H),
            other => Err(anyhow!(
                "unknown QR level '{other}' (want L, M, Q, or H)"
            )),
        }
    }

    pub(crate) fn as_ec(self) -> qrcode::EcLevel {
        match self {
            QrLevel::L => qrcode::EcLevel::L,
            QrLevel::M => qrcode::EcLevel::M,
            QrLevel::Q => qrcode::EcLevel::Q,
            QrLevel::H => qrcode::EcLevel::H,
        }
    }
}


/// Options that tune individual encoders. Empty/default today; extended
/// as features land. Most callers use `EncodeOptions::default()`.
#[derive(Debug, Clone, Default)]
pub struct EncodeOptions {
    pub qr_level: QrLevel,
    /// rxing `--encode-hints KEY=VAL` pairs (already parsed but not yet
    /// validated against the per-format hint enum). Empty when the user
    /// passed no hints. Currently honoured only by `encode_via_rxing`
    /// (Aztec / PDF417); a hint set on a non-rxing format errors at the
    /// top of `encode_with_opts`.
    pub rxing_hints: Vec<(String, String)>,
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
    encode_with_opts(format, input, &EncodeOptions::default())
}

/// Like `encode`, but threads per-encoder tuning from `EncodeOptions`.
pub fn encode_with_opts(
    format: Format,
    input: &[u8],
    opts: &EncodeOptions,
) -> Result<BitMatrix> {
    if !opts.rxing_hints.is_empty()
        && !matches!(format, Format::Aztec | Format::Pdf417)
    {
        return Err(anyhow!(
            "--encode-hints currently applies only to aztec / pdf417 (recon's other encoders \
             use crates without a hint API); got format {}",
            format.canonical()
        ));
    }
    match format {
        Format::Qr => encode_qr(input, opts.qr_level),
        Format::DataMatrix => encode_datamatrix(input),
        Format::Code128 => encode_1d(format, input),
        Format::Code39 => encode_1d(format, input),
        Format::Ean13 => encode_1d(format, input),
        Format::UpcA => encode_1d(format, input),
        Format::Aztec => encode_via_rxing(format, input, &opts.rxing_hints),
        Format::Pdf417 => encode_via_rxing(format, input, &opts.rxing_hints),
    }
}

fn encode_via_rxing(
    format: Format,
    input: &[u8],
    hint_pairs: &[(String, String)],
) -> Result<BitMatrix> {
    use rxing::{BarcodeFormat, Writer};

    let bf = match format {
        Format::Aztec => BarcodeFormat::AZTEC,
        Format::Pdf417 => BarcodeFormat::PDF_417,
        _ => return Err(anyhow!("encode_via_rxing: unsupported format {:?}", format)),
    };
    let text = std::str::from_utf8(input)
        .map_err(|e| anyhow!("{}: payload must be UTF-8 text ({e})", format.canonical()))?;

    // rxing requires a hinted target size; pass 0 for square formats
    // and a rectangular hint for PDF417 (width ≥ height).
    let (w, h) = match format {
        Format::Pdf417 => (300, 120),
        _ => (0, 0),
    };

    let hints = build_rxing_hints(format, hint_pairs)?;
    let writer = rxing::MultiFormatWriter;
    let rxing_matrix = writer
        .encode_with_hints(text, &bf, w, h, &hints)
        .map_err(|e| anyhow!("{} encode error: {e:?}", format.canonical()))?;

    let width = rxing_matrix.getWidth();
    let height = rxing_matrix.getHeight();
    let mut bits = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            bits.push(rxing_matrix.get(x, y));
        }
    }
    Ok(BitMatrix {
        width,
        height,
        bits,
        kind: format.kind(),
    })
}

/// Parse a single `--encode-hints KEY=VAL` token into `(key, value)`.
/// Key is lowercased and trimmed; value is the raw RHS (no trimming so
/// a hint like `charset= ` with a trailing space is still rejected by
/// the rxing layer rather than silently scrubbed).
pub fn parse_hint_kv(s: &str) -> Result<(String, String)> {
    let (k, v) = s.split_once('=').ok_or_else(|| {
        anyhow!("--encode-hints: expected KEY=VAL, got `{s}`")
    })?;
    let key = k.trim().to_ascii_lowercase();
    if key.is_empty() {
        return Err(anyhow!("--encode-hints: empty key in `{s}`"));
    }
    Ok((key, v.to_string()))
}

/// Translate the parsed CLI hint pairs into rxing's strongly-typed
/// `EncodeHints` struct. Only the keys explicitly documented on
/// `--encode-hints` are accepted; anything else errors so a typo
/// fails loud instead of being silently dropped.
fn build_rxing_hints(
    format: Format,
    pairs: &[(String, String)],
) -> Result<rxing::EncodeHints> {
    let mut hints = rxing::EncodeHints::default();
    for (k, v) in pairs {
        match k.as_str() {
            "charset" => hints.CharacterSet = Some(v.clone()),
            "eclevel" => hints.ErrorCorrection = Some(v.clone()),
            "aztec-layers" => {
                if !matches!(format, Format::Aztec) {
                    return Err(anyhow!(
                        "--encode-hints aztec-layers only applies to --encode aztec"
                    ));
                }
                let n: i32 = v.parse().map_err(|_| {
                    anyhow!("--encode-hints aztec-layers: expected integer (-4..-1 compact, 0 auto, 1..32 full), got `{v}`")
                })?;
                hints.AztecLayers = Some(n);
            }
            "pdf417-compact" => {
                require_pdf417(format, k)?;
                hints.Pdf417Compact = Some(v.clone());
            }
            "pdf417-compaction" => {
                require_pdf417(format, k)?;
                hints.Pdf417Compaction = Some(v.clone());
            }
            "pdf417-auto-eci" => {
                require_pdf417(format, k)?;
                hints.Pdf417AutoEci = Some(v.clone());
            }
            "margin" => hints.Margin = Some(v.clone()),
            other => {
                return Err(anyhow!(
                    "--encode-hints: unknown key `{other}` (supported: charset, eclevel, \
                     aztec-layers, pdf417-compact, pdf417-compaction, pdf417-auto-eci, margin)"
                ));
            }
        }
    }
    Ok(hints)
}

fn require_pdf417(format: Format, key: &str) -> Result<()> {
    if matches!(format, Format::Pdf417) {
        Ok(())
    } else {
        Err(anyhow!(
            "--encode-hints {key} only applies to --encode pdf417"
        ))
    }
}

fn encode_qr(input: &[u8], level: QrLevel) -> Result<BitMatrix> {
    let qr = qrcode::QrCode::with_error_correction_level(input, level.as_ec())
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

/// Validate an EAN-style numeric input and return the 12-digit string that
/// barcoders expects. barcoders EAN13/UPCA always takes exactly 12 digits
/// (the body without the check digit it encodes as the 13th bar group).
///
/// `body_len` is the user-visible body length (digits WITHOUT the check):
///   * 12 for EAN-13 — accept 12 digits (pass through) or 13 digits (strip check).
///   * 11 for UPC-A  — accept 11 digits (append computed check) or 12 digits (pass through).
///
/// In both cases the returned string is exactly 12 digits.
fn prepare_ean_like(text: &str, body_len: usize, name: &str) -> Result<String> {
    if !text.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("{name}: input must be digits only"));
    }
    match text.len() {
        n if n == body_len && body_len < 12 => {
            // Short form that needs a check digit appended to reach 12 (UPC-A: 11 → 12).
            let check = ean_check_digit(text.as_bytes());
            Ok(format!("{text}{check}"))
        }
        n if n == body_len && body_len == 12 => {
            // EAN-13 body (12 digits): already exactly what barcoders wants.
            Ok(text.to_string())
        }
        n if n == body_len + 1 && n > 12 => {
            // EAN-13 full code (13 digits): strip the check — barcoders recomputes it.
            Ok(text[..12].to_string())
        }
        n if n == body_len + 1 && n == 12 => {
            // UPC-A full code (12 digits): already exactly what barcoders wants.
            Ok(text.to_string())
        }
        _ => Err(anyhow!(
            "{name}: input must be {body_len} or {} digits; got {}",
            body_len + 1,
            text.len(),
        )),
    }
}

/// Mod-10 check digit used by EAN-13, UPC-A, UPC-E and ITF.
/// `body` is the ASCII-digit sequence WITHOUT the check digit.
/// Algorithm: from the rightmost body digit, multiply digits alternately by
/// 3 and 1, sum, then the check is (10 - sum % 10) % 10.
fn ean_check_digit(body: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    // From rightmost to leftmost; rightmost gets weight 3.
    for (i, &b) in body.iter().rev().enumerate() {
        let d = (b - b'0') as u32;
        let w = if i % 2 == 0 { 3 } else { 1 };
        sum += d * w;
    }
    let check = (10 - (sum % 10)) % 10;
    b'0' + (check as u8)
}

fn encode_1d(format: Format, input: &[u8]) -> Result<BitMatrix> {
    let text = std::str::from_utf8(input)
        .map_err(|_| anyhow!("{}: input must be valid UTF-8", format.canonical()))?
        .trim();

    let bars: Vec<u8> = match format {
        Format::Code128 => {
            // barcoders' Code128 requires a leading code-set marker: À (U+00C0)
            // for A, Ɓ (U+0181) for B, Ć (U+0106) for C. Most inputs are mixed
            // ASCII, which is code-set B; prepend Ɓ unless the input already
            // selects a code-set explicitly.
            let prepared = if text.starts_with('À') || text.starts_with('Ɓ') || text.starts_with('Ć') {
                text.to_string()
            } else {
                format!("Ɓ{text}")
            };
            let bc = barcoders::sym::code128::Code128::new(&prepared)
                .map_err(|e| anyhow!("code128: {e}"))?;
            bc.encode()
        }
        Format::Code39 => {
            let bc = barcoders::sym::code39::Code39::new(text)
                .map_err(|e| anyhow!("code39: {e}"))?;
            bc.encode()
        }
        Format::Ean13 => {
            let prepared = prepare_ean_like(text, 12, "ean13")?;
            let bc = barcoders::sym::ean13::EAN13::new(&prepared)
                .map_err(|e| anyhow!("ean13: {e}"))?;
            bc.encode()
        }
        Format::UpcA => {
            let prepared = prepare_ean_like(text, 11, "upca")?;
            let bc = barcoders::sym::ean13::UPCA::new(&prepared)
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

// ---- Renderers — Task 4 ------------------------------------------------

use std::io::Write;

// Renderer constants.
const TWOD_PIXEL_SCALE: u32 = 8;       // 8× per module for 2D codes
const ONED_PIXEL_WIDTH: u32 = 2;       // 2× horizontal stretch for 1D
const ONED_BAR_HEIGHT: u32 = 50;       // row count for 1D output
const QUIET_ZONE_MODULES: u32 = 2;     // blank modules around the matrix

/// Render to unicode-block ASCII.
/// - 2D codes: pair two rows per line with `▀`/`▄`/`█`/` ` half-blocks so output
///   looks square-ish in the terminal.
/// - 1D codes: extrude the single row into a tall block using full blocks.
pub fn render_ascii(matrix: &BitMatrix) -> String {
    render_ascii_with_hrt(matrix, None)
}

/// Like `render_ascii`, but optionally prints a human-readable text
/// (HRT) line below the barcode. Centered if it fits; left-aligned
/// otherwise.
pub fn render_ascii_with_hrt(matrix: &BitMatrix, hrt: Option<&str>) -> String {
    let mut out = match matrix.kind {
        MatrixKind::TwoD => render_ascii_2d(matrix),
        MatrixKind::OneD => render_ascii_1d(matrix),
    };
    if let Some(text) = hrt {
        let total_cols = (matrix.width + 2 * QUIET_ZONE_MODULES) as usize;
        let pad = total_cols.saturating_sub(text.chars().count()) / 2;
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(text);
        out.push('\n');
    }
    out
}

fn render_ascii_2d(matrix: &BitMatrix) -> String {
    let upper_lower = |up: bool, lo: bool| match (up, lo) {
        (false, false) => ' ',
        (true, false) => '▀',
        (false, true) => '▄',
        (true, true) => '█',
    };
    let mut out = String::new();
    let qz = QUIET_ZONE_MODULES as i64;
    let w = matrix.width as i64;
    let h = matrix.height as i64;
    let mut y = -qz;
    while y < h + qz {
        // quiet zone left
        for _ in 0..qz {
            out.push(upper_lower(false, false));
        }
        for x in 0..w {
            let up = if y >= 0 && y < h { matrix.get(x as u32, y as u32) } else { false };
            let lo_y = y + 1;
            let lo = if lo_y >= 0 && lo_y < h { matrix.get(x as u32, lo_y as u32) } else { false };
            out.push(upper_lower(up, lo));
        }
        // quiet zone right
        for _ in 0..qz {
            out.push(upper_lower(false, false));
        }
        out.push('\n');
        y += 2;
    }
    out
}

fn render_ascii_1d(matrix: &BitMatrix) -> String {
    // Single row tiled vertically to a few lines for visibility.
    const LINES: u32 = 6;
    let mut out = String::new();
    let qz = QUIET_ZONE_MODULES;
    for _ in 0..LINES {
        for _ in 0..qz {
            out.push(' ');
        }
        for x in 0..matrix.width {
            out.push(if matrix.get(x, 0) { '█' } else { ' ' });
        }
        for _ in 0..qz {
            out.push(' ');
        }
        out.push('\n');
    }
    out
}

/// Render to a self-contained SVG document. Black-on-white.
pub fn render_svg(matrix: &BitMatrix) -> String {
    render_svg_with_hrt(matrix, None)
}

/// Like `render_svg`, but optionally emits an HRT `<text>` element
/// below the barcode body. Font-family is left to the rendering
/// environment (sans-serif by default).
pub fn render_svg_with_hrt(matrix: &BitMatrix, hrt: Option<&str>) -> String {
    let (scale, height_mul) = match matrix.kind {
        MatrixKind::TwoD => (TWOD_PIXEL_SCALE, 1),
        MatrixKind::OneD => (ONED_PIXEL_WIDTH, ONED_BAR_HEIGHT),
    };
    let qz = QUIET_ZONE_MODULES;
    let module_w = matrix.width + 2 * qz;
    // Extra space below for HRT when present: ~1.2x the per-module
    // pixel scale gives a readable row under the bars.
    let hrt_px = if hrt.is_some() {
        (scale as f32 * 1.6) as u32
    } else {
        0
    };
    let module_h = (matrix.height * height_mul) + 2 * qz;
    let px_w = module_w * scale;
    let px_h = module_h * scale + hrt_px;

    let mut out = String::new();
    out.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" \
              viewBox=\"0 0 {px_w} {px_h}\" \
              width=\"{px_w}\" height=\"{px_h}\" \
              shape-rendering=\"crispEdges\">\n\
           <rect width=\"{px_w}\" height=\"{px_h}\" fill=\"white\"/>\n"
    ));

    match matrix.kind {
        MatrixKind::TwoD => {
            for y in 0..matrix.height {
                for x in 0..matrix.width {
                    if matrix.get(x, y) {
                        let px_x = (x + qz) * scale;
                        let px_y = (y + qz) * scale;
                        out.push_str(&format!(
                            "  <rect x=\"{px_x}\" y=\"{px_y}\" width=\"{scale}\" height=\"{scale}\" fill=\"black\"/>\n"
                        ));
                    }
                }
            }
        }
        MatrixKind::OneD => {
            let bar_px_h = ONED_BAR_HEIGHT * scale;
            let bar_y = qz * scale;
            for x in 0..matrix.width {
                if matrix.get(x, 0) {
                    let px_x = (x + qz) * scale;
                    out.push_str(&format!(
                        "  <rect x=\"{px_x}\" y=\"{bar_y}\" width=\"{scale}\" height=\"{bar_px_h}\" fill=\"black\"/>\n"
                    ));
                }
            }
        }
    }

    if let Some(text) = hrt {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let text_x = px_w / 2;
        let text_y = module_h * scale + (hrt_px * 3) / 4;
        let font_px = ((scale as f32) * 1.2) as u32;
        out.push_str(&format!(
            "  <text x=\"{text_x}\" y=\"{text_y}\" \
              text-anchor=\"middle\" \
              font-family=\"monospace, sans-serif\" \
              font-size=\"{font_px}\" \
              fill=\"black\">{escaped}</text>\n"
        ));
    }

    out.push_str("</svg>\n");
    out
}

/// Render to a PNG byte stream. Grayscale 8-bit (0 = black, 255 = white).
pub fn render_png(matrix: &BitMatrix) -> Result<Vec<u8>> {
    render_png_with_hrt(matrix, None)
}

/// Bundled DejaVu Sans Mono. Bitstream Vera + DejaVu license; see
/// `assets/fonts/LICENSE-DejaVu.txt`. Used to rasterize PNG HRT;
/// SVG output leaves font selection to the renderer.
const HRT_FONT_TTF: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");

/// Minimum HRT band height in pixels for PNG output. Picked so a 14 px
/// glyph stays readable even at 1D's 2-pixel-per-module bar scale.
const HRT_PNG_MIN_BAND_PX: u32 = 22;

/// Like `render_png`, but optionally rasterizes an HRT line below the
/// barcode body using the bundled DejaVu Sans Mono font.
pub fn render_png_with_hrt(matrix: &BitMatrix, hrt: Option<&str>) -> Result<Vec<u8>> {
    let (scale, height_mul) = match matrix.kind {
        MatrixKind::TwoD => (TWOD_PIXEL_SCALE, 1),
        MatrixKind::OneD => (ONED_PIXEL_WIDTH, ONED_BAR_HEIGHT),
    };
    let qz = QUIET_ZONE_MODULES;
    let module_w = matrix.width + 2 * qz;
    let module_h = (matrix.height * height_mul) + 2 * qz;
    let px_w = module_w * scale;
    // Pick an HRT band tall enough to fit a legible monospace glyph
    // at any per-module scale. The SVG impl can lean on the renderer
    // to upscale; PNG is raw pixels, so we floor the band height at
    // ~22 px so 14 px glyphs (with room for descenders) stay readable
    // even when 1D `scale = 2` would otherwise give a 3 px band.
    let hrt_px = if hrt.is_some() {
        ((scale as f32 * 1.6) as u32).max(HRT_PNG_MIN_BAND_PX)
    } else {
        0
    };
    let px_h = module_h * scale + hrt_px;

    // One byte per pixel grayscale (0 = black, 255 = white).
    let mut pixels: Vec<u8> = vec![255u8; (px_w * px_h) as usize];
    let set_pixel = |pixels: &mut Vec<u8>, x: u32, y: u32| {
        let idx = (y * px_w + x) as usize;
        pixels[idx] = 0;
    };

    match matrix.kind {
        MatrixKind::TwoD => {
            for my in 0..matrix.height {
                for mx in 0..matrix.width {
                    if matrix.get(mx, my) {
                        let px_x0 = (mx + qz) * scale;
                        let px_y0 = (my + qz) * scale;
                        for dy in 0..scale {
                            for dx in 0..scale {
                                set_pixel(&mut pixels, px_x0 + dx, px_y0 + dy);
                            }
                        }
                    }
                }
            }
        }
        MatrixKind::OneD => {
            let bar_px_h = ONED_BAR_HEIGHT * scale;
            let bar_y0 = qz * scale;
            for mx in 0..matrix.width {
                if matrix.get(mx, 0) {
                    let px_x0 = (mx + qz) * scale;
                    for dy in 0..bar_px_h {
                        for dx in 0..scale {
                            set_pixel(&mut pixels, px_x0 + dx, bar_y0 + dy);
                        }
                    }
                }
            }
        }
    }

    if let Some(text) = hrt {
        rasterize_hrt(&mut pixels, px_w, px_h, module_h * scale, hrt_px, text)?;
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, px_w, px_h);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| anyhow!("png: {e}"))?;
        writer.write_image_data(&pixels).map_err(|e| anyhow!("png: {e}"))?;
    }
    Ok(out)
}

/// Rasterize `text` centered horizontally in the HRT band below the
/// barcode. Pixel band runs from y=barcode_px_h to y=barcode_px_h+hrt_px.
fn rasterize_hrt(
    pixels: &mut [u8],
    px_w: u32,
    _px_h: u32,
    barcode_px_h: u32,
    hrt_px: u32,
    text: &str,
) -> Result<()> {
    use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

    let font = FontRef::try_from_slice(HRT_FONT_TTF)
        .map_err(|e| anyhow!("hrt: failed to load bundled font: {e}"))?;

    // Match the SVG font sizing convention: the HRT band is ~1.6x
    // the per-module scale; rendering at ~75% of the band gives a
    // comfortable line.
    let font_size = (hrt_px as f32) * 0.75;
    let scaled = font.as_scaled(PxScale::from(font_size));

    // First pass: measure total advance to centre the string.
    let mut advance = 0.0_f32;
    let mut prev = None::<char>;
    for c in text.chars() {
        let glyph_id = scaled.glyph_id(c);
        if let Some(p) = prev {
            advance += scaled.kern(scaled.glyph_id(p), glyph_id);
        }
        advance += scaled.h_advance(glyph_id);
        prev = Some(c);
    }

    let start_x = ((px_w as f32) - advance) / 2.0;
    // Vertical: place the baseline ~80% down the HRT band so
    // descenders fit without clipping.
    let baseline_y = barcode_px_h as f32 + (hrt_px as f32) * 0.80;

    let mut cursor_x = start_x;
    let mut prev = None::<char>;
    for c in text.chars() {
        let glyph_id = scaled.glyph_id(c);
        if let Some(p) = prev {
            cursor_x += scaled.kern(scaled.glyph_id(p), glyph_id);
        }
        let glyph = glyph_id.with_scale_and_position(
            PxScale::from(font_size),
            ab_glyph::point(cursor_x, baseline_y),
        );
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || (px as u32) >= px_w {
                    return;
                }
                let idx = (py as u32 * px_w + px as u32) as usize;
                if let Some(slot) = pixels.get_mut(idx) {
                    // Blend toward black by the glyph coverage (0..1).
                    let bg = *slot as f32;
                    let blended = bg * (1.0 - coverage);
                    *slot = blended as u8;
                }
            });
        }
        cursor_x += scaled.h_advance(glyph_id);
        prev = Some(c);
    }
    Ok(())
}

/// Print `--encode-list` to the given writer.
pub fn print_list(out: &mut dyn Write) -> std::io::Result<()> {
    for f in Format::ALL {
        let kind = match f.kind() {
            MatrixKind::TwoD => "2D",
            MatrixKind::OneD => "1D",
        };
        writeln!(out, "{:<11} {}  {}", f.canonical(), kind, f.description())?;
    }
    Ok(())
}

/// Resolve the encode input. Priority: `--from-file` > positional `'-'` or
/// pipe > positional literal > error. Reads as bytes so DataMatrix can carry
/// arbitrary content.
pub fn resolve_input(
    from_file: Option<&Path>,
    positional: &str,
) -> Result<Vec<u8>> {
    if let Some(path) = from_file {
        if !positional.is_empty() && positional != "-" {
            return Err(anyhow!(
                "--from-file and a positional text are mutually exclusive"
            ));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| anyhow!("failed to read '{}': {e}", path.display()))?;
        let trimmed = if bytes.ends_with(b"\n") { &bytes[..bytes.len() - 1] } else { &bytes[..] };
        return Ok(trimmed.to_vec());
    }
    if positional == "-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut buf)?;
        let trimmed = if buf.ends_with(b"\n") { &buf[..buf.len() - 1] } else { &buf[..] };
        return Ok(trimmed.to_vec());
    }
    if positional.is_empty() {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut buf)?;
            let trimmed = if buf.ends_with(b"\n") { &buf[..buf.len() - 1] } else { &buf[..] };
            return Ok(trimmed.to_vec());
        }
        return Err(anyhow!(
            "--encode requires a positional text, --from-file <PATH>, or a pipe on stdin"
        ));
    }
    Ok(positional.as_bytes().to_vec())
}

use crate::cli::Args;

/// Top-level dispatcher for `--encode`.
pub fn run(args: &Args) -> Result<()> {
    let fmt_str = args.encode.as_deref().unwrap_or("");
    let format = parse_format(fmt_str)?;

    let out_format = resolve_output_format(
        args.encode_format.as_deref(),
        args.output.as_deref(),
    )?;

    let input = resolve_input(args.from_file.as_deref(), args.target_url())?;

    let rxing_hints = args
        .encode_hints
        .iter()
        .map(|s| parse_hint_kv(s))
        .collect::<Result<Vec<_>>>()?;
    let opts = EncodeOptions {
        qr_level: QrLevel::parse(&args.qr_level)?,
        rxing_hints,
    };
    let matrix = encode_with_opts(format, &input, &opts)?;

    if args.verbose >= 1 {
        let of_label = match out_format {
            OutputFormat::Ascii => "ascii",
            OutputFormat::Svg => "svg",
            OutputFormat::Png => "png",
        };
        eprintln!(
            "* encode: {} -> {} ({}x{} modules)",
            format.canonical(),
            of_label,
            matrix.width,
            matrix.height,
        );
    }

    // HRT decision: explicit --no-hrt wins; else --hrt wins; else
    // default-on for EAN-13 / UPC-A, off for the others.
    let hrt_text: Option<&str> = if args.no_hrt {
        None
    } else if args.hrt || matches!(format, Format::Ean13 | Format::UpcA) {
        std::str::from_utf8(&input).ok()
    } else {
        None
    };

    let bytes: Vec<u8> = match out_format {
        OutputFormat::Ascii => render_ascii_with_hrt(&matrix, hrt_text).into_bytes(),
        OutputFormat::Svg => render_svg_with_hrt(&matrix, hrt_text).into_bytes(),
        OutputFormat::Png => render_png_with_hrt(&matrix, hrt_text)?,
    };

    match &args.output {
        Some(path) => {
            let mut file = std::fs::File::create(path)?;
            file.write_all(&bytes)?;
        }
        None => {
            std::io::stdout().lock().write_all(&bytes)?;
        }
    }
    Ok(())
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
        let err = parse_format("absolutely-not-a-format").unwrap_err().to_string();
        assert!(err.contains("absolutely-not-a-format"), "got: {err}");
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
    fn format_all_has_all_variants() {
        assert_eq!(Format::ALL.len(), 8);
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
        let m = encode(Format::Code128, b"RECON-TEST-001").unwrap();
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
    fn encode_upca_11_digits_ok() {
        let m = encode(Format::UpcA, b"01234567890").unwrap();
        assert_eq!(m.kind, MatrixKind::OneD);
    }

    #[test]
    fn ean_check_digit_known_vectors() {
        // Canonical examples: EAN-13 "5901234123457" has body "590123412345"
        // with check digit 7. UPC-A "012345678905" has body "01234567890"
        // with check digit 5.
        assert_eq!(ean_check_digit(b"590123412345"), b'7');
        assert_eq!(ean_check_digit(b"01234567890"), b'5');
    }

    fn tiny_2d_matrix() -> BitMatrix {
        // 3×3 all dark except center.
        BitMatrix {
            width: 3,
            height: 3,
            bits: vec![
                true, true, true,
                true, false, true,
                true, true, true,
            ],
            kind: MatrixKind::TwoD,
        }
    }

    #[test]
    fn render_ascii_2d_produces_output_lines() {
        let out = render_ascii(&tiny_2d_matrix());
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() >= 2, "expected at least 2 output lines; got {}\n{out}", lines.len());
        assert!(!out.is_empty());
    }

    #[test]
    fn render_ascii_1d_produces_multiple_lines() {
        let m = BitMatrix {
            width: 5,
            height: 1,
            bits: vec![true, false, true, false, true],
            kind: MatrixKind::OneD,
        };
        let out = render_ascii(&m);
        assert!(out.lines().count() >= 4, "1D should render multiple lines:\n{out}");
    }

    #[test]
    fn render_svg_well_formed() {
        let out = render_svg(&tiny_2d_matrix());
        assert!(out.starts_with("<?xml"), "got: {}", &out[..30.min(out.len())]);
        assert!(out.contains("<svg "), "missing <svg>: {out}");
        assert!(out.trim_end().ends_with("</svg>"), "no closing </svg>: {out}");
        let rect_count = out.matches("<rect").count();
        assert!(rect_count >= 8, "expected ≥8 <rect>s, got {rect_count}:\n{out}");
    }

    #[test]
    fn render_svg_1d_has_one_rect_per_bar() {
        let m = BitMatrix {
            width: 3,
            height: 1,
            bits: vec![true, false, true],
            kind: MatrixKind::OneD,
        };
        let out = render_svg(&m);
        // 1 background rect + 2 bar rects = 3
        assert_eq!(out.matches("<rect").count(), 3, "got:\n{out}");
    }

    #[test]
    fn render_png_has_signature_and_ihdr() {
        let bytes = render_png(&tiny_2d_matrix()).unwrap();
        assert!(bytes.len() > 16);
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&bytes[12..16], b"IHDR");
    }

    #[test]
    fn render_png_decodes_back() {
        let bytes = render_png(&tiny_2d_matrix()).unwrap();
        let decoder = png::Decoder::new(bytes.as_slice());
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        // 3 modules + 2 quiet-zones on each side = 7 modules wide; × 8 scale = 56.
        assert_eq!(info.width, 56);
        assert_eq!(info.height, 56);
    }

    #[test]
    fn run_qr_to_ascii_via_cli_args() {
        use clap::Parser;

        let args = Args::try_parse_from([
            "recon",
            "--encode",
            "qr",
            "hello",
        ]).unwrap();

        // Run writes to stdout but should at least succeed for a clean input.
        assert!(run(&args).is_ok());
    }

    #[test]
    fn run_qr_to_file_writes_png() {
        use clap::Parser;

        let tmp = std::env::temp_dir().join(format!(
            "recon-encode-test-{}.png",
            std::process::id()
        ));

        let args = Args::try_parse_from([
            "recon",
            "--encode",
            "qr",
            "-o",
            tmp.to_str().unwrap(),
            "https://example.com",
        ]).unwrap();
        run(&args).unwrap();

        let bytes = std::fs::read(&tmp).unwrap();
        assert!(bytes.len() > 100, "expected a PNG file of some size");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn print_list_has_all_lines() {
        let mut out = Vec::new();
        print_list(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 8, "got:\n{text}");
        assert!(text.contains("qr"));
        assert!(text.contains("datamatrix"));
        assert!(text.contains("upca"));
        assert!(text.contains("aztec"));
        assert!(text.contains("pdf417"));
    }

    #[test]
    fn resolve_input_errors_on_both_file_and_positional() {
        let path = std::path::PathBuf::from("/tmp/does-not-matter");
        let err = resolve_input(Some(&path), "some text").unwrap_err().to_string();
        assert!(err.contains("mutually exclusive"), "got: {err}");
    }

    #[test]
    fn parse_hint_kv_basic() {
        let (k, v) = parse_hint_kv("charset=UTF-8").unwrap();
        assert_eq!(k, "charset");
        assert_eq!(v, "UTF-8");
    }

    #[test]
    fn parse_hint_kv_lowercases_key_preserves_value() {
        let (k, v) = parse_hint_kv("Charset=Shift_JIS").unwrap();
        assert_eq!(k, "charset");
        assert_eq!(v, "Shift_JIS");
    }

    #[test]
    fn parse_hint_kv_value_can_contain_equals() {
        let (k, v) = parse_hint_kv("eclevel=a=b").unwrap();
        assert_eq!(k, "eclevel");
        assert_eq!(v, "a=b");
    }

    #[test]
    fn parse_hint_kv_errors_without_equals() {
        let err = parse_hint_kv("nokvhere").unwrap_err().to_string();
        assert!(err.contains("KEY=VAL"), "got: {err}");
    }

    #[test]
    fn parse_hint_kv_errors_on_empty_key() {
        let err = parse_hint_kv("=value").unwrap_err().to_string();
        assert!(err.contains("empty key"), "got: {err}");
    }

    #[test]
    fn encode_hints_aztec_layers_changes_matrix_size() {
        let auto = encode_with_opts(
            Format::Aztec,
            b"aztec test",
            &EncodeOptions::default(),
        ).unwrap();
        let opts = EncodeOptions {
            qr_level: QrLevel::default(),
            rxing_hints: vec![("aztec-layers".into(), "-2".into())],
        };
        let compact = encode_with_opts(Format::Aztec, b"aztec test", &opts).unwrap();
        assert_ne!(auto.width, compact.width, "compact aztec should differ in size from auto-layers");
    }

    #[test]
    fn encode_hints_rejects_unknown_key() {
        let opts = EncodeOptions {
            qr_level: QrLevel::default(),
            rxing_hints: vec![("bogus".into(), "v".into())],
        };
        let err = encode_with_opts(Format::Aztec, b"x", &opts).unwrap_err().to_string();
        assert!(err.contains("unknown key"), "got: {err}");
    }

    #[test]
    fn encode_hints_rejects_on_non_rxing_format() {
        let opts = EncodeOptions {
            qr_level: QrLevel::default(),
            rxing_hints: vec![("charset".into(), "UTF-8".into())],
        };
        let err = encode_with_opts(Format::Qr, b"x", &opts).unwrap_err().to_string();
        assert!(err.contains("aztec / pdf417"), "got: {err}");
    }

    #[test]
    fn encode_hints_aztec_layers_rejected_on_pdf417() {
        let opts = EncodeOptions {
            qr_level: QrLevel::default(),
            rxing_hints: vec![("aztec-layers".into(), "1".into())],
        };
        let err = encode_with_opts(Format::Pdf417, b"x", &opts).unwrap_err().to_string();
        assert!(err.contains("aztec-layers only applies to --encode aztec"), "got: {err}");
    }
}
