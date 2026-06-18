//! Document-conversion script bindings — `md_to_html`, `md_to_pdf`,
//! `html_to_pdf`.
//!
//! Scripts already have `http()` / `file_read()` for source loading;
//! these bindings take content directly (string or Blob). PDF
//! generation requires agent-browser on PATH; a missing binary is
//! reported with the same install guidance as the CLI flags.

use crate::docs::{markdown_to_html, DocOptions};
use crate::script::convert::err;
use rhai::{Blob, Engine, EvalAltResult, Map};
use std::path::PathBuf;

fn opts_from_map(m: &Map) -> Result<DocOptions, Box<EvalAltResult>> {
    let mut opts = DocOptions {
        toc_depth: 3,
        toc_title: "Contents".into(),
        page_size: "a4".into(),
        page_numbers: true,
        ..DocOptions::default()
    };
    if let Some(v) = m.get("toc") {
        opts.toc = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("toc_depth") {
        if let Ok(n) = v.as_int() {
            opts.toc_depth = n.clamp(1, 6) as u8;
        }
    }
    if let Some(v) = m.get("toc_title") {
        if let Ok(s) = v.clone().into_string() {
            opts.toc_title = s;
        }
    }
    if let Some(v) = m.get("title") {
        if let Ok(s) = v.clone().into_string() {
            opts.title = Some(s);
        }
    }
    if let Some(v) = m.get("author") {
        if let Ok(s) = v.clone().into_string() {
            opts.author = Some(s);
        }
    }
    if let Some(v) = m.get("subject") {
        if let Ok(s) = v.clone().into_string() {
            opts.subject = Some(s);
        }
    }
    if let Some(v) = m.get("keywords") {
        if let Ok(s) = v.clone().into_string() {
            opts.keywords = Some(s);
        }
    }
    if let Some(v) = m.get("css") {
        if let Ok(s) = v.clone().into_string() {
            opts.custom_css = Some(s);
        }
    }
    if let Some(v) = m.get("no_default_css") {
        opts.no_default_css = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("gfm") {
        opts.gfm = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("unsafe_html") {
        opts.unsafe_html = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("page_break_on_h1") {
        opts.page_break_on_h1 = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("pdf_engine") {
        if let Ok(s) = v.clone().into_string() {
            opts.pdf_engine = match s.to_ascii_lowercase().as_str() {
                "chrome" => crate::cli::PdfEngine::Chrome,
                _ => crate::cli::PdfEngine::Typst,
            };
        }
    }
    if let Some(v) = m.get("page_size") {
        if let Ok(s) = v.clone().into_string() {
            opts.page_size = s;
        }
    }
    if let Some(v) = m.get("cover") {
        opts.cover = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = m.get("cover_template") {
        if let Ok(s) = v.clone().into_string() {
            opts.cover_template = Some(std::path::PathBuf::from(s));
        }
    }
    if let Some(v) = m.get("doc_subtitle") {
        if let Ok(s) = v.clone().into_string() {
            opts.subtitle = Some(s);
        }
    }
    if let Some(v) = m.get("doc_version") {
        if let Ok(s) = v.clone().into_string() {
            opts.version = Some(s);
        }
    }
    if let Some(v) = m.get("doc_date") {
        if let Ok(s) = v.clone().into_string() {
            opts.date = Some(s);
        }
    }
    if let Some(v) = m.get("no_page_numbers") {
        if v.as_bool().unwrap_or(false) {
            opts.page_numbers = false;
        }
    }
    if let Some(v) = m.get("font") {
        if let Ok(s) = v.clone().into_string() {
            opts.font = Some(s);
        }
    }
    if let Some(v) = m.get("font_path") {
        // Accept either a single directory string or an array of strings.
        if let Ok(s) = v.clone().into_string() {
            opts.font_path.push(s);
        } else if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
            for item in arr {
                if let Ok(s) = item.into_string() {
                    opts.font_path.push(s);
                }
            }
        }
    }
    Ok(opts)
}

/// Dispatch markdown bytes → PDF using the engine chosen in `opts`.
///
/// Typst path: parse the AST in-process and call the native renderer.
/// Chrome path: convert to HTML first, then hand off to agent-browser.
fn render_md_bytes_to_pdf(md: &[u8], dest: &str, opts: &DocOptions) -> Result<(), Box<EvalAltResult>> {
    match opts.pdf_engine {
        crate::cli::PdfEngine::Typst => {
            use comrak::{parse_document, Arena};
            use crate::docs::comrak_options;
            let source = std::str::from_utf8(md).map_err(|e| err(e.to_string()))?;
            let arena = Arena::new();
            let comrak_opts = comrak_options(opts);
            let root = parse_document(&arena, source, &comrak_opts);
            let base_dir = std::env::current_dir().map_err(|e| err(e.to_string()))?;
            let http = reqwest::blocking::Client::new();
            let pdf_bytes = crate::typst_pdf::render_md_to_pdf(root, opts, &base_dir, &http)
                .map_err(|e| err(e.to_string()))?;
            std::fs::write(dest, &pdf_bytes).map_err(|e| err(e.to_string()))
        }
        crate::cli::PdfEngine::Chrome => {
            let html = markdown_to_html(md, opts).map_err(|e| err(e.to_string()))?;
            crate::docs_pdf::render_html_to_pdf(html.as_bytes(), &PathBuf::from(dest))
                .map_err(|e| err(e.to_string()))
        }
    }
}

pub fn register(engine: &mut Engine) {
    // md_to_html(md_str) / md_to_html(md_blob) → HTML string
    engine.register_fn(
        "md_to_html",
        |md: &str| -> Result<String, Box<EvalAltResult>> {
            markdown_to_html(md.as_bytes(), &DocOptions::default())
                .map_err(|e| err(e.to_string()))
        },
    );
    engine.register_fn(
        "md_to_html",
        |md: Blob| -> Result<String, Box<EvalAltResult>> {
            markdown_to_html(&md, &DocOptions::default()).map_err(|e| err(e.to_string()))
        },
    );
    engine.register_fn(
        "md_to_html",
        |md: &str, opts: Map| -> Result<String, Box<EvalAltResult>> {
            let opts = opts_from_map(&opts)?;
            markdown_to_html(md.as_bytes(), &opts).map_err(|e| err(e.to_string()))
        },
    );
    engine.register_fn(
        "md_to_html",
        |md: Blob, opts: Map| -> Result<String, Box<EvalAltResult>> {
            let opts = opts_from_map(&opts)?;
            markdown_to_html(&md, &opts).map_err(|e| err(e.to_string()))
        },
    );

    // html_to_text(html) — render HTML to text with safe defaults
    // (no ANSI styling, terminal/80-column width). Deterministic by
    // default so script output is stable in pipelines and tests.
    engine.register_fn(
        "html_to_text",
        |html: &str| -> Result<String, Box<EvalAltResult>> {
            let opts = crate::render::RenderOpts {
                width: None,
                color: crate::cli::ColorWhen::Never,
                no_links: false,
            };
            crate::render::render_html(html, &opts).map_err(|e| err(e.to_string()))
        },
    );
    // html_to_text(html, #{ width, color }) — `width` is a number,
    // `color` is one of "always" / "auto" / "never" (default "never").
    engine.register_fn(
        "html_to_text",
        |html: &str, opts: Map| -> Result<String, Box<EvalAltResult>> {
            let width = opts
                .get("width")
                .and_then(|v| v.as_int().ok())
                .map(|n| n.max(0) as usize);
            let color = match opts
                .get("color")
                .and_then(|v| v.clone().into_string().ok())
                .as_deref()
            {
                Some("always") => crate::cli::ColorWhen::Always,
                Some("auto") => crate::cli::ColorWhen::Auto,
                _ => crate::cli::ColorWhen::Never,
            };
            let no_links = opts
                .get("no_links")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false);
            let ropts = crate::render::RenderOpts { width, color, no_links };
            crate::render::render_html(html, &ropts).map_err(|e| err(e.to_string()))
        },
    );

    // html_to_pdf(html_str, dest_path)
    engine.register_fn(
        "html_to_pdf",
        |html: &str, dest: &str| -> Result<(), Box<EvalAltResult>> {
            crate::docs_pdf::render_html_to_pdf(html.as_bytes(), &PathBuf::from(dest))
                .map_err(|e| err(e.to_string()))
        },
    );
    engine.register_fn(
        "html_to_pdf",
        |html: Blob, dest: &str| -> Result<(), Box<EvalAltResult>> {
            crate::docs_pdf::render_html_to_pdf(&html, &PathBuf::from(dest))
                .map_err(|e| err(e.to_string()))
        },
    );

    // md_to_pdf(md_str, dest_path [, opts])
    engine.register_fn(
        "md_to_pdf",
        |md: &str, dest: &str| -> Result<(), Box<EvalAltResult>> {
            let opts = DocOptions {
                toc_depth: 3,
                toc_title: "Contents".into(),
                page_size: "a4".into(),
                page_numbers: true,
                ..DocOptions::default()
            };
            render_md_bytes_to_pdf(md.as_bytes(), dest, &opts)
        },
    );
    engine.register_fn(
        "md_to_pdf",
        |md: &str, dest: &str, opts: Map| -> Result<(), Box<EvalAltResult>> {
            let opts = opts_from_map(&opts)?;
            render_md_bytes_to_pdf(md.as_bytes(), dest, &opts)
        },
    );
    engine.register_fn(
        "md_to_pdf",
        |md: Blob, dest: &str, opts: Map| -> Result<(), Box<EvalAltResult>> {
            let opts = opts_from_map(&opts)?;
            render_md_bytes_to_pdf(&md, dest, &opts)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_to_html_basic() {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        let html: String = e
            .eval("md_to_html(\"# Hello\\n\\nBody.\\n\")")
            .expect("eval");
        assert!(html.contains("<h1"), "html: {html}");
        assert!(html.contains("Hello"), "html: {html}");
    }

    #[test]
    fn md_to_html_with_toc() {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        let html: String = e
            .eval(
                "md_to_html(\"# Intro\\n\\n## Setup\\n\\nBody.\\n\", #{ toc: true, toc_depth: 2 })",
            )
            .expect("eval");
        assert!(html.contains("<nav class=\"toc\">"), "html: {html}");
        assert!(html.contains("href=\"#setup\""), "html: {html}");
    }
}
