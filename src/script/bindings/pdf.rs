//! `pdf_export_page(pdf_path, page, [dest], [opts])` — render a single
//! PDF page to PNG / JPEG / WEBP. Mirrors the CLI `--export-pdf-page`
//! flag; relies on `pdf_export::render_page` for the agent-browser
//! drive sequence.
//!
//! Overloads:
//!   pdf_export_page(pdf, page)                 -> Blob (PNG by default)
//!   pdf_export_page(pdf, page, opts)           -> Blob
//!   pdf_export_page(pdf, page, dest)           -> ()  writes to dest
//!   pdf_export_page(pdf, page, dest, opts)     -> ()
//!
//! Opts map keys: viewport ("WxH"), scale (int), quality (int 0-100),
//! format ("png"|"jpeg"|"webp").

use crate::pdf_export::{infer_format, parse_viewport, render_page, RenderOpts};
#[cfg(test)]
use crate::pdf_export::OutputFormat;
use crate::script::convert::err;
use rhai::{Blob, Engine, EvalAltResult, Map};
use std::path::Path;

fn opts_from_map(m: &Map, dest_path: Option<&Path>) -> Result<RenderOpts, Box<EvalAltResult>> {
    let mut o = RenderOpts::default();
    if let Some(v) = m.get("viewport") {
        if let Ok(s) = v.clone().into_string() {
            let (w, h) = parse_viewport(&s).map_err(|e| err(e.to_string()))?;
            o.viewport_w = w;
            o.viewport_h = h;
        }
    }
    if let Some(v) = m.get("scale") {
        if let Ok(n) = v.as_int() {
            if n < 1 {
                return Err(err("pdf_export_page: scale must be >= 1".to_string()));
            }
            o.scale = n as u32;
        }
    }
    if let Some(v) = m.get("quality") {
        if let Ok(n) = v.as_int() {
            o.quality = n.clamp(0, 100) as u32;
        }
    }
    let explicit_fmt = m
        .get("format")
        .and_then(|v| v.clone().into_string().ok());
    o.format = infer_format(explicit_fmt.as_deref(), dest_path).map_err(|e| err(e.to_string()))?;
    Ok(o)
}

fn run_to_blob(pdf: &str, page: i64, opts: Option<&Map>) -> Result<Blob, Box<EvalAltResult>> {
    let mut render_opts = match opts {
        Some(m) => opts_from_map(m, None)?,
        None => RenderOpts::default(),
    };
    if page < 1 {
        return Err(err("pdf_export_page: page must be >= 1".to_string()));
    }
    render_opts.page = page as u32;
    let bytes =
        render_page(Path::new(pdf), &render_opts).map_err(|e| err(e.to_string()))?;
    let mut blob = Blob::new();
    blob.extend_from_slice(&bytes);
    Ok(blob)
}

fn run_to_dest(
    pdf: &str,
    page: i64,
    dest: &str,
    opts: Option<&Map>,
) -> Result<(), Box<EvalAltResult>> {
    let dest_path = std::path::PathBuf::from(dest);
    let mut render_opts = match opts {
        Some(m) => opts_from_map(m, Some(&dest_path))?,
        None => {
            let format = infer_format(None, Some(&dest_path))
                .map_err(|e| err(e.to_string()))?;
            RenderOpts { format, ..Default::default() }
        }
    };
    if page < 1 {
        return Err(err("pdf_export_page: page must be >= 1".to_string()));
    }
    render_opts.page = page as u32;
    let bytes =
        render_page(Path::new(pdf), &render_opts).map_err(|e| err(e.to_string()))?;
    std::fs::write(&dest_path, &bytes)
        .map_err(|e| err(format!("pdf_export_page: write '{}': {e}", dest_path.display())))
}

pub fn register(engine: &mut Engine) {
    engine.register_fn(
        "pdf_export_page",
        |pdf: &str, page: i64| -> Result<Blob, Box<EvalAltResult>> {
            run_to_blob(pdf, page, None)
        },
    );
    engine.register_fn(
        "pdf_export_page",
        |pdf: &str, page: i64, third: rhai::Dynamic| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            // Third arg may be a dest string OR an opts map.
            if third.clone().try_cast::<Map>().is_some() {
                let map: Map = third.cast();
                let blob = run_to_blob(pdf, page, Some(&map))?;
                Ok(rhai::Dynamic::from_blob(blob))
            } else if third.clone().try_cast::<String>().is_some() {
                let dest: String = third.cast();
                run_to_dest(pdf, page, &dest, None)?;
                Ok(rhai::Dynamic::UNIT)
            } else {
                Err(err(
                    "pdf_export_page: third argument must be a destination \
                     path (string) or an opts map".to_string(),
                ))
            }
        },
    );
    engine.register_fn(
        "pdf_export_page",
        |pdf: &str, page: i64, dest: &str, opts: Map| -> Result<(), Box<EvalAltResult>> {
            run_to_dest(pdf, page, dest, Some(&opts))
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opts_from_map_defaults() {
        let m = Map::new();
        let opts = opts_from_map(&m, None).unwrap();
        assert_eq!(opts.viewport_w, 1024);
        assert_eq!(opts.viewport_h, 1366);
        assert_eq!(opts.scale, 2);
        assert_eq!(opts.quality, 90);
        assert!(matches!(opts.format, OutputFormat::Png));
    }

    #[test]
    fn opts_from_map_parses_viewport_and_scale() {
        let mut m = Map::new();
        m.insert("viewport".into(), "1920x2715".into());
        m.insert("scale".into(), 3i64.into());
        m.insert("quality".into(), 75i64.into());
        m.insert("format".into(), "webp".into());
        let opts = opts_from_map(&m, None).unwrap();
        assert_eq!(opts.viewport_w, 1920);
        assert_eq!(opts.viewport_h, 2715);
        assert_eq!(opts.scale, 3);
        assert_eq!(opts.quality, 75);
        assert!(matches!(opts.format, OutputFormat::Webp));
    }

    #[test]
    fn opts_from_map_dest_extension_picks_format() {
        let m = Map::new();
        let dest = std::path::PathBuf::from("out.jpg");
        let opts = opts_from_map(&m, Some(&dest)).unwrap();
        assert!(matches!(opts.format, OutputFormat::Jpeg));
    }
}
