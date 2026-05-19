//! `--export-pdf-page` — render a single PDF page to a raster image
//! through `agent-browser` (Chrome). See the design spec for the full
//! flow; this module owns the option parsing, format dispatch, and the
//! agent-browser drive sequence.

use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::path::Path;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Jpeg,
    Webp,
}

impl OutputFormat {
    pub fn from_name(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpeg" | "jpg" => Ok(Self::Jpeg),
            "webp" => Ok(Self::Webp),
            other => Err(anyhow!(
                "unknown output format '{other}'; use png, jpeg, or webp"
            )),
        }
    }

    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }

    pub fn default_extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
}

/// Resolve the output format from `--pdf-format` (explicit) then path
/// extension then the default `Png`. Errors only when an explicit
/// `--pdf-format` is unknown or the path's extension is unknown AND no
/// explicit override is set.
pub fn infer_format(explicit: Option<&str>, path: Option<&Path>) -> Result<OutputFormat> {
    if let Some(name) = explicit {
        return OutputFormat::from_name(name);
    }
    if let Some(p) = path {
        if let Some(fmt) = OutputFormat::from_extension(p) {
            return Ok(fmt);
        }
        // Has a path but extension is unrecognised — only reject when the
        // path has an extension. A bare filename "out" can fall through to
        // the default PNG.
        if p.extension().is_some() {
            return Err(anyhow!(
                "could not infer output format from '{}'; pass --pdf-format png|jpeg|webp",
                p.display()
            ));
        }
    }
    Ok(OutputFormat::Png)
}

/// All knobs for a render. Constructed by the CLI entry point or the
/// Rhai binding.
#[derive(Debug, Clone)]
pub struct RenderOpts {
    pub page: u32,
    pub viewport_w: u32,
    pub viewport_h: u32,
    pub scale: u32,
    pub quality: u32,
    pub format: OutputFormat,
}

impl Default for RenderOpts {
    fn default() -> Self {
        Self {
            page: 1,
            viewport_w: 1024,
            viewport_h: 1366,
            scale: 2,
            quality: 90,
            format: OutputFormat::Png,
        }
    }
}

/// Decode a PNG buffer and re-encode it as WEBP at the given quality
/// (0–100). Quality > 100 is clamped to 100.
pub fn png_to_webp(png_bytes: &[u8], quality: u32) -> Result<Vec<u8>> {
    use png::Decoder;
    let decoder = Decoder::new(png_bytes);
    let mut reader = decoder.read_info().context("png decode header")?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).context("png decode frame")?;
    // `webp::Encoder::from_rgba` / `from_rgb` expects a tightly-packed
    // buffer. Convert grayscale / palette / 16-bit by re-using the
    // `png` crate's transformations: here we only support the formats
    // Chrome's screenshot emits, which is always RGBA8.
    if info.bit_depth != png::BitDepth::Eight {
        return Err(anyhow!(
            "pdf-export: expected 8-bit PNG from agent-browser, got {:?}",
            info.bit_depth
        ));
    }
    let q = quality.min(100) as f32;
    let webp_bytes = match info.color_type {
        png::ColorType::Rgba => {
            let enc = webp::Encoder::from_rgba(&buf[..info.buffer_size()], info.width, info.height);
            enc.encode(q).to_vec()
        }
        png::ColorType::Rgb => {
            let enc = webp::Encoder::from_rgb(&buf[..info.buffer_size()], info.width, info.height);
            enc.encode(q).to_vec()
        }
        other => {
            return Err(anyhow!(
                "pdf-export: unsupported PNG color type {:?} from agent-browser",
                other
            ));
        }
    };
    Ok(webp_bytes)
}

use std::path::PathBuf;
use std::process::Command;

/// Whether `pdftoppm` is reachable on PATH. Used by the binding and by
/// integration tests that gate on the renderer's availability.
pub fn pdftoppm_available() -> bool {
    Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|o| o.status.success() || !o.stderr.is_empty())
        .unwrap_or(false)
}

/// Per-page geometry returned by `pdfinfo`.
struct PageInfo {
    total_pages: u32,
    width_pts: f64,
    height_pts: f64,
}

/// Run `pdfinfo` (optionally limited to a single page) and capture
/// stdout. `page` is `None` for a top-level call (returns total pages
/// and page-1 size) or `Some(N)` to ask for page N's geometry.
fn run_pdfinfo(pdf: &Path, page: Option<u32>) -> Result<String> {
    let mut cmd = Command::new("pdfinfo");
    if let Some(n) = page {
        let s = n.to_string();
        cmd.args(["-f", &s, "-l", &s]);
    }
    let out = cmd.arg(pdf).output().context("spawn pdfinfo")?;
    if !out.status.success() {
        return Err(anyhow!(
            "pdfinfo failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Pull `Pages: <n>` from a pdfinfo dump.
fn parse_total_pages(s: &str) -> Option<u32> {
    s.lines()
        .find_map(|l| l.strip_prefix("Pages:"))
        .and_then(|rest| rest.trim().parse().ok())
}

/// Pull `Page <n> size:  W x H pts (...)` from a pdfinfo dump.
fn parse_page_size(s: &str) -> Option<(f64, f64)> {
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("Page") {
            if let Some(idx) = rest.find("size:") {
                let geom = &rest[idx + "size:".len()..];
                if let Some(pts_idx) = geom.find("pts") {
                    let pair = geom[..pts_idx].trim();
                    if let Some((w, h)) = pair.split_once('x') {
                        if let (Ok(w), Ok(h)) =
                            (w.trim().parse::<f64>(), h.trim().parse::<f64>())
                        {
                            return Some((w, h));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Resolve total page count + the requested page's geometry. The
/// total-pages query runs first so we can reject out-of-range pages
/// with a clean message before pdfinfo's per-page call complains
/// about "Wrong page range given".
fn pdfinfo_page(pdf: &Path, page: u32) -> Result<PageInfo> {
    let top = run_pdfinfo(pdf, None)?;
    let total_pages =
        parse_total_pages(&top).context("pdfinfo: Pages: line missing")?;
    if page > total_pages {
        return Err(anyhow!(
            "page {} out of range: PDF has {} page(s)",
            page,
            total_pages
        ));
    }
    // Page sizes are usually uniform; ask pdfinfo for this specific
    // page's geometry so mixed-size PDFs render correctly.
    let per_page = run_pdfinfo(pdf, Some(page))?;
    let (width_pts, height_pts) = parse_page_size(&per_page)
        .or_else(|| parse_page_size(&top))
        .context("pdfinfo: page size line missing")?;
    if width_pts <= 0.0 || height_pts <= 0.0 {
        return Err(anyhow!(
            "pdfinfo: page {page} has invalid dimensions {width_pts} x {height_pts}"
        ));
    }
    Ok(PageInfo {
        total_pages,
        width_pts,
        height_pts,
    })
}

/// Render `opts.page` of the PDF at `pdf_path` and return the bytes in
/// the format specified by `opts.format`. Caller is responsible for
/// writing the bytes to disk or stdout.
///
/// Flow: pdfinfo for page geometry → compute fit-within DPI from
/// `viewport × scale` → `pdftoppm -singlefile -r DPI` → read result →
/// transcode to WEBP if requested. Aspect ratio is preserved; the
/// `viewport` × `scale` rectangle is treated as an upper bound, and
/// the page is scaled so the longer dimension matches.
pub fn render_page(pdf_path: &Path, opts: &RenderOpts) -> Result<Vec<u8>> {
    if !pdftoppm_available() {
        return Err(anyhow!(
            "PDF-page export needs `pdftoppm` (from poppler-utils). \
             Install via `brew install poppler` (macOS) or \
             `apt install poppler-utils` (Debian/Ubuntu) and retry."
        ));
    }

    if opts.page == 0 {
        return Err(anyhow!("page number must be >= 1"));
    }
    if opts.scale == 0 {
        return Err(anyhow!("--pdf-scale must be >= 1"));
    }

    let abs = pdf_path
        .canonicalize()
        .with_context(|| format!("PDF not found: {}", pdf_path.display()))?;

    let info = pdfinfo_page(&abs, opts.page)?;
    if opts.page > info.total_pages {
        return Err(anyhow!(
            "page {} out of range: PDF has {} page(s)",
            opts.page,
            info.total_pages
        ));
    }

    // Fit-within DPI: pick the smaller of the two so neither dimension
    // overflows the requested box. 72 pts = 1 inch.
    let box_w = (opts.viewport_w as u64 * opts.scale as u64) as f64;
    let box_h = (opts.viewport_h as u64 * opts.scale as u64) as f64;
    let dpi_x = box_w * 72.0 / info.width_pts;
    let dpi_y = box_h * 72.0 / info.height_pts;
    let dpi = dpi_x.min(dpi_y).max(1.0).round() as u32;

    let capture_fmt = match opts.format {
        OutputFormat::Webp => OutputFormat::Png, // transcode after
        other => other,
    };

    let tmpdir = tempfile::Builder::new()
        .prefix("recon-pdf-page-")
        .tempdir()
        .context("create render tempdir")?;
    let prefix_path = tmpdir.path().join("page");
    let prefix_str = prefix_path.to_str().context("tempdir path is not UTF-8")?;
    let abs_str = abs.to_str().context("PDF path is not UTF-8")?;
    let page_str = opts.page.to_string();
    let dpi_str = dpi.to_string();

    let mut args: Vec<String> = vec![
        "-f".into(),
        page_str.clone(),
        "-l".into(),
        page_str,
        "-singlefile".into(),
        "-r".into(),
        dpi_str,
    ];
    let jpeg_quality_arg;
    match capture_fmt {
        OutputFormat::Png => args.push("-png".into()),
        OutputFormat::Jpeg => {
            args.push("-jpeg".into());
            jpeg_quality_arg = format!("quality={}", opts.quality.min(100));
            args.push("-jpegopt".into());
            args.push(jpeg_quality_arg);
        }
        OutputFormat::Webp => unreachable!("captured as png"),
    }
    args.push(abs_str.to_string());
    args.push(prefix_str.to_string());

    let out = Command::new("pdftoppm")
        .args(&args)
        .output()
        .context("spawn pdftoppm")?;
    if !out.status.success() {
        return Err(anyhow!(
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let out_ext = match capture_fmt {
        OutputFormat::Png => "png",
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Webp => unreachable!(),
    };
    let out_path = prefix_path.with_extension(out_ext);
    let captured = std::fs::read(&out_path).with_context(|| {
        format!("read pdftoppm output '{}'", out_path.display())
    })?;

    if matches!(opts.format, OutputFormat::Webp) {
        png_to_webp(&captured, opts.quality)
    } else {
        Ok(captured)
    }
}

/// CLI entry point for `--export-pdf-page`.
pub fn run_export_pdf_page_cli(args: &crate::cli::Args) -> Result<()> {
    let pair = args
        .export_pdf_page
        .as_ref()
        .context("--export-pdf-page requires two values: PAGE PDF")?;
    if pair.len() != 2 {
        return Err(anyhow!(
            "--export-pdf-page takes exactly two values: <PAGE> <PDF>"
        ));
    }
    let page: u32 = pair[0]
        .parse()
        .with_context(|| format!("page number '{}' is not a positive integer", pair[0]))?;
    let pdf = PathBuf::from(&pair[1]);
    if !pdf.exists() {
        return Err(anyhow!("PDF not found: {}", pdf.display()));
    }

    let (vw, vh) = match args.pdf_viewport.as_deref() {
        Some(s) => parse_viewport(s)?,
        None => (1024u32, 1366u32),
    };
    let scale = args.pdf_scale.unwrap_or(2);
    let quality = args.pdf_quality.unwrap_or(90);

    // Format resolution: explicit --pdf-format, then -o extension, then PNG.
    let out_path_for_inference: Option<&Path> = args
        .output
        .as_ref()
        .filter(|p| p.to_str() != Some("-"))
        .map(|p| p.as_path());
    let format = infer_format(args.pdf_format.as_deref(), out_path_for_inference)?;

    let opts = RenderOpts {
        page,
        viewport_w: vw,
        viewport_h: vh,
        scale,
        quality,
        format,
    };
    let bytes = render_page(&pdf, &opts)?;

    // Output: -o PATH (where "-" is stdout), or default page-<N>.<ext>.
    let dest = args.output.as_ref();
    match dest {
        Some(p) if p.to_str() == Some("-") => {
            let mut out = std::io::stdout().lock();
            out.write_all(&bytes).context("write stdout")?;
        }
        Some(p) => {
            std::fs::write(p, &bytes)
                .with_context(|| format!("write '{}'", p.display()))?;
        }
        None => {
            let default = format!("page-{}.{}", page, format.default_extension());
            std::fs::write(&default, &bytes)
                .with_context(|| format!("write '{default}'"))?;
        }
    }
    Ok(())
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

    #[test]
    fn infer_format_from_extension() {
        use std::path::PathBuf;
        assert_eq!(
            infer_format(None, Some(&PathBuf::from("x.png"))).unwrap(),
            OutputFormat::Png
        );
        assert_eq!(
            infer_format(None, Some(&PathBuf::from("x.jpg"))).unwrap(),
            OutputFormat::Jpeg
        );
        assert_eq!(
            infer_format(None, Some(&PathBuf::from("x.JPEG"))).unwrap(),
            OutputFormat::Jpeg
        );
        assert_eq!(
            infer_format(None, Some(&PathBuf::from("x.webp"))).unwrap(),
            OutputFormat::Webp
        );
    }

    #[test]
    fn infer_format_explicit_overrides() {
        use std::path::PathBuf;
        assert_eq!(
            infer_format(Some("webp"), Some(&PathBuf::from("x.png"))).unwrap(),
            OutputFormat::Webp
        );
    }

    #[test]
    fn infer_format_unknown_extension_errors() {
        use std::path::PathBuf;
        let e = infer_format(None, Some(&PathBuf::from("x.gif")))
            .unwrap_err()
            .to_string();
        assert!(e.contains("--pdf-format"), "got: {e}");
    }

    #[test]
    fn infer_format_default_png_when_no_extension() {
        use std::path::PathBuf;
        assert_eq!(
            infer_format(None, Some(&PathBuf::from("out"))).unwrap(),
            OutputFormat::Png
        );
        assert_eq!(infer_format(None, None).unwrap(), OutputFormat::Png);
    }

    #[test]
    fn output_format_from_name_unknown() {
        assert!(OutputFormat::from_name("gif").is_err());
    }

    #[test]
    fn png_to_webp_roundtrip() {
        // Build a 2x2 RGBA PNG in memory.
        let mut png_bytes: Vec<u8> = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            // 2x2 = 16 bytes RGBA, alternating red and blue.
            let pixels: [u8; 16] = [
                255, 0, 0, 255,  0, 0, 255, 255,
                0, 0, 255, 255,  255, 0, 0, 255,
            ];
            writer.write_image_data(&pixels).unwrap();
        }
        let webp_bytes = png_to_webp(&png_bytes, 80).expect("webp encode");
        // WEBP files start with "RIFF" then 4 size bytes then "WEBP".
        assert_eq!(&webp_bytes[0..4], b"RIFF", "bytes: {:02x?}", &webp_bytes[..16]);
        assert_eq!(&webp_bytes[8..12], b"WEBP");
    }
}
