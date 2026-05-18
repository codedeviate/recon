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

/// Render `opts.page` of the PDF at `pdf_path` and return the bytes in
/// the format specified by `opts.format`. Caller is responsible for
/// writing the bytes to disk or stdout.
///
/// Flow: set viewport → open file://…#page=N → wait → screenshot → close.
/// Always closes the agent-browser session, even on error, so a hung
/// invocation doesn't leak a session.
pub fn render_page(pdf_path: &Path, opts: &RenderOpts) -> Result<Vec<u8>> {
    let state = crate::agent_browser::state_snapshot();
    if !state.available {
        return Err(anyhow!(
            "PDF-page export needs agent-browser (which renders the PDF in Chrome). \
             Install via `brew install agent-browser` or \
             `npm install -g agent-browser` and retry."
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
    let abs_str = abs.to_str().context("PDF path is not UTF-8")?;
    let url = format!(
        "file://{abs_str}#page={page}&toolbar=0&navpanes=0&scrollbar=0&view=Fit",
        page = opts.page
    );

    // Output for agent-browser screenshot. We need a tempfile because
    // agent-browser writes to a path; we read it back as bytes.
    let capture_fmt = match opts.format {
        OutputFormat::Webp => OutputFormat::Png, // transcode after
        other => other,
    };
    let tmp = tempfile::Builder::new()
        .prefix("recon-pdf-page-")
        .suffix(&format!(".{}", capture_fmt.default_extension()))
        .tempfile()
        .context("create capture tempfile")?;
    let tmp_path: PathBuf = tmp.path().to_path_buf();
    let tmp_str = tmp_path.to_str().context("tempfile path is not UTF-8")?;

    let w = opts.viewport_w.to_string();
    let h = opts.viewport_h.to_string();
    let scale = opts.scale.to_string();
    let quality = opts.quality.min(100).to_string();
    let wait_ms = "500"; // see spec §6 — Chrome layout settle time

    let drive = || -> Result<()> {
        crate::agent_browser::run_cmd(&["set", "viewport", &w, &h, &scale], false)
            .context("agent-browser set viewport")?;
        crate::agent_browser::run_cmd(&["open", &url], false)
            .context("agent-browser open")?;
        // wait <ms>: agent-browser's wait takes either a selector or a
        // numeric millisecond count.
        let _ = crate::agent_browser::run_cmd(&["wait", wait_ms], false);
        let fmt_arg = match capture_fmt {
            OutputFormat::Png => "png",
            OutputFormat::Jpeg => "jpeg",
            OutputFormat::Webp => unreachable!("captured as png"),
        };
        let mut shot_args: Vec<&str> = vec![
            "screenshot",
            "--full",
            "--screenshot-format",
            fmt_arg,
        ];
        if matches!(capture_fmt, OutputFormat::Jpeg) {
            shot_args.push("--screenshot-quality");
            shot_args.push(&quality);
        }
        shot_args.push(tmp_str);
        crate::agent_browser::run_cmd(&shot_args, false)
            .context("agent-browser screenshot")?;
        Ok(())
    };

    let result = drive();
    // Always close.
    let _ = crate::agent_browser::run_cmd(&["close"], false);
    result?;

    let captured = std::fs::read(&tmp_path).context("read screenshot tempfile")?;
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
