//! Document conversions: markdown → HTML, markdown → PDF, HTML → PDF.
//!
//! HTML rendering is pure-Rust via `comrak` (CommonMark + GFM). PDF
//! rendering goes through the existing `agent-browser` CLI integration
//! (`agent-browser open file://... && agent-browser pdf <path>`) which
//! preserves anchor links, @page CSS, and produces a clickable TOC.

use anyhow::{Context, Result};
use comrak::{
    nodes::{AstNode, NodeValue},
    parse_document, Arena, ExtensionOptions, Options, RenderOptions,
};
use std::path::Path;

use crate::cli::Args;

const DEFAULT_CSS: &str = r#"
@page { size: A4; margin: 18mm 20mm; }
body {
    font-family: -apple-system, "Segoe UI", "Helvetica Neue", Arial, sans-serif;
    font-size: 11pt;
    line-height: 1.5;
    color: #111;
    max-width: 780px;
    margin: 0 auto;
    padding: 0 8px;
}
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin-top: 1.6em; margin-bottom: 0.4em; }
h1 { font-size: 24pt; border-bottom: 1px solid #ddd; padding-bottom: 0.2em; }
h2 { font-size: 18pt; border-bottom: 1px solid #eee; padding-bottom: 0.15em; }
h3 { font-size: 14pt; }
code, pre, kbd, samp { font-family: "SF Mono", Menlo, Consolas, monospace; font-size: 10pt; }
pre {
    background: #f5f5f5;
    padding: 10px 12px;
    border-radius: 4px;
    overflow-x: auto;
    page-break-inside: avoid;
}
code { background: #f5f5f5; padding: 1px 4px; border-radius: 3px; }
pre code { background: transparent; padding: 0; }
blockquote {
    border-left: 3px solid #ccc;
    margin: 0.8em 0;
    padding: 0.2em 1em;
    color: #555;
}
table { border-collapse: collapse; margin: 0.8em 0; }
th, td { border: 1px solid #ccc; padding: 4px 10px; }
th { background: #f0f0f0; }
a { color: #0366d6; text-decoration: none; }
a:hover { text-decoration: underline; }
img { max-width: 100%; height: auto; }
hr { border: 0; border-top: 1px solid #ddd; margin: 2em 0; }
nav.toc {
    background: #fafafa;
    border: 1px solid #eee;
    border-radius: 4px;
    padding: 10px 16px;
    margin: 0 0 1.6em 0;
    page-break-after: avoid;
}
nav.toc h2 { margin: 0.2em 0 0.4em 0; border: 0; font-size: 14pt; }
nav.toc ul { list-style: none; padding-left: 1em; margin: 0.2em 0; }
nav.toc > ul { padding-left: 0; }
nav.toc a { color: #0366d6; }
"#;

/// Options for document conversion. Parsed from CLI flags or opts map.
#[derive(Debug, Clone, Default)]
pub struct DocOptions {
    pub toc: bool,
    pub toc_depth: u8,
    pub toc_title: String,
    pub title: Option<String>,
    pub custom_css: Option<String>,
    pub no_default_css: bool,
    pub gfm: bool,
}

impl DocOptions {
    pub fn from_args(args: &Args) -> Result<Self> {
        let custom_css = match args.doc_css.as_ref() {
            Some(path) => Some(
                std::fs::read_to_string(path)
                    .with_context(|| format!("--doc-css: read {}", path.display()))?,
            ),
            None => None,
        };
        Ok(Self {
            toc: args.toc,
            toc_depth: if args.toc_depth == 0 { 3 } else { args.toc_depth },
            toc_title: args.toc_title.clone(),
            title: args.doc_title.clone(),
            custom_css,
            no_default_css: args.no_default_css,
            gfm: args.gfm,
        })
    }
}

/// Markdown bytes → fully-wrapped HTML document string.
pub fn markdown_to_html(markdown: &[u8], opts: &DocOptions) -> Result<String> {
    let source = std::str::from_utf8(markdown).context("markdown is not valid UTF-8")?;

    let arena = Arena::new();
    let comrak_opts = comrak_options(opts);
    let root = parse_document(&arena, source, &comrak_opts);

    let mut toc_html = String::new();
    if opts.toc {
        let headings = collect_headings(root, opts.toc_depth);
        if !headings.is_empty() {
            toc_html = render_toc(&headings, &opts.toc_title);
        }
    }

    let mut body = Vec::new();
    comrak::format_html(root, &comrak_opts, &mut body)
        .context("comrak: format_html")?;
    let body_str = String::from_utf8(body).context("comrak output is not UTF-8")?;

    let title = opts.title.clone().unwrap_or_else(|| "Document".into());
    Ok(wrap_document(&title, opts, &toc_html, &body_str))
}

fn comrak_options(opts: &DocOptions) -> Options {
    let mut ext = ExtensionOptions::default();
    ext.header_ids = Some(String::new()); // emit <h1 id="slug"> for TOC links

    if opts.gfm {
        ext.strikethrough = true;
        ext.tagfilter = true;
        ext.table = true;
        ext.autolink = true;
        ext.tasklist = true;
        ext.footnotes = true;
    } else {
        // Conservative: tables + strikethrough + tasklist + autolink
        // are useful enough to default-on even without --gfm.
        ext.strikethrough = true;
        ext.table = true;
        ext.autolink = true;
        ext.tasklist = true;
    }

    let mut render = RenderOptions::default();
    render.unsafe_ = false;

    Options {
        extension: ext,
        parse: Default::default(),
        render,
    }
}

struct Heading {
    level: u8,
    text: String,
    id: String,
}

fn collect_headings<'a>(root: &'a AstNode<'a>, max_depth: u8) -> Vec<Heading> {
    let mut out = Vec::new();
    for node in root.descendants() {
        if let NodeValue::Heading(h) = &node.data.borrow().value {
            if h.level > max_depth {
                continue;
            }
            let text = node_text(node);
            if text.is_empty() {
                continue;
            }
            let id = slugify(&text);
            out.push(Heading {
                level: h.level,
                text,
                id,
            });
        }
    }
    out
}

fn node_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

fn collect_text<'a>(node: &'a AstNode<'a>, out: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::Code(c) => out.push_str(&c.literal),
        _ => {
            for child in node.children() {
                collect_text(child, out);
            }
        }
    }
}

/// Slugify to match comrak's default header-id generation as closely
/// as possible: lowercase, ASCII alphanumerics + dashes, spaces →
/// dashes, drop punctuation.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' || c == '_' {
            out.push('-');
        }
        // other characters are dropped
    }
    // Collapse consecutive dashes.
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn render_toc(headings: &[Heading], title: &str) -> String {
    let mut s = String::new();
    s.push_str("<nav class=\"toc\">\n");
    s.push_str(&format!("  <h2>{}</h2>\n", html_escape(title)));
    s.push_str("  <ul>\n");
    let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);
    let mut current_level = min_level;
    for h in headings {
        while current_level < h.level {
            s.push_str("<ul>\n");
            current_level += 1;
        }
        while current_level > h.level {
            s.push_str("</ul>\n");
            current_level -= 1;
        }
        s.push_str(&format!(
            "<li><a href=\"#{}\">{}</a></li>\n",
            html_escape(&h.id),
            html_escape(&h.text)
        ));
    }
    while current_level > min_level {
        s.push_str("</ul>\n");
        current_level -= 1;
    }
    s.push_str("  </ul>\n</nav>\n");
    s
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn wrap_document(title: &str, opts: &DocOptions, toc_html: &str, body_html: &str) -> String {
    let mut css = String::new();
    if !opts.no_default_css {
        css.push_str(DEFAULT_CSS);
    }
    if let Some(extra) = opts.custom_css.as_deref() {
        css.push('\n');
        css.push_str(extra);
    }

    format!(
        "<!doctype html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <title>{title}</title>\n\
  <style>\n{css}\n  </style>\n\
</head>\n\
<body>\n\
<main>\n\
{toc}{body}\n\
</main>\n\
</body>\n\
</html>\n",
        title = html_escape(title),
        css = css,
        toc = toc_html,
        body = body_html,
    )
}

/// CLI entry point for `--md-to-html`.
pub fn run_md_to_html(args: &Args) -> Result<()> {
    let src = args
        .md_to_html
        .as_ref()
        .context("--md-to-html requires a source (path / URL / -)")?;
    let opts = DocOptions::from_args(args)?;
    let bytes = load_source(args, src)?;
    let html = markdown_to_html(&bytes, &opts)?;
    write_output(args, html.as_bytes())
}

/// CLI entry point for `--md-to-pdf`.
pub fn run_md_to_pdf(args: &Args) -> Result<()> {
    let src = args
        .md_to_pdf
        .as_ref()
        .context("--md-to-pdf requires a source (path / URL / -)")?;
    let output = args
        .output
        .as_ref()
        .context("--md-to-pdf requires -o <PATH>")?;
    let opts = DocOptions::from_args(args)?;
    let bytes = load_source(args, src)?;
    let html = markdown_to_html(&bytes, &opts)?;
    crate::docs_pdf::render_html_to_pdf(html.as_bytes(), output)
}

/// CLI entry point for `--html-to-pdf`.
pub fn run_html_to_pdf(args: &Args) -> Result<()> {
    let src = args
        .html_to_pdf
        .as_ref()
        .context("--html-to-pdf requires a source (path / URL / -)")?;
    let output = args
        .output
        .as_ref()
        .context("--html-to-pdf requires -o <PATH>")?;
    let bytes = load_source(args, src)?;
    crate::docs_pdf::render_html_to_pdf(&bytes, output)
}

fn load_source(args: &Args, src: &str) -> Result<Vec<u8>> {
    let mut call_args = args.clone();
    call_args.md_to_html = None;
    call_args.md_to_pdf = None;
    call_args.html_to_pdf = None;
    call_args.url = Some(src.to_string());
    call_args.url_flag = None;
    crate::source::read_all(&call_args)
}

fn write_output(args: &Args, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    match args.output.as_ref() {
        Some(path) => {
            std::fs::write(path, bytes)
                .with_context(|| format!("write output '{}'", path.display()))?;
        }
        None => {
            std::io::stdout()
                .lock()
                .write_all(bytes)
                .context("write stdout")?;
        }
    }
    Ok(())
}

pub(crate) fn _unused(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_ids_emitted() {
        let opts = DocOptions::default();
        let html = markdown_to_html(b"# Hello World\n\nBody.\n", &opts).unwrap();
        assert!(html.contains("id=\"hello-world\""), "html: {html}");
    }

    #[test]
    fn toc_generated_when_enabled() {
        let mut opts = DocOptions::default();
        opts.toc = true;
        opts.toc_depth = 3;
        opts.toc_title = "Contents".into();
        let md = b"# Intro\n\n## Setup\n\nBody.\n\n## Usage\n\nMore.\n";
        let html = markdown_to_html(md, &opts).unwrap();
        assert!(html.contains("<nav class=\"toc\">"), "html: {html}");
        assert!(html.contains("href=\"#setup\""), "html: {html}");
        assert!(html.contains("href=\"#usage\""), "html: {html}");
        assert!(html.contains("<h2>Contents</h2>"), "html: {html}");
    }

    #[test]
    fn toc_depth_limit_honored() {
        let mut opts = DocOptions::default();
        opts.toc = true;
        opts.toc_depth = 1;
        let md = b"# A\n\n## B\n\n### C\n";
        let html = markdown_to_html(md, &opts).unwrap();
        // Isolate just the <nav class="toc">…</nav> block; comrak's
        // heading-id emission wraps each heading in its own
        // <a href="#slug"> in the body, so a naive contains() would
        // also match those.
        let toc_start = html.find("<nav class=\"toc\">").expect("nav");
        let toc_end = html[toc_start..].find("</nav>").expect("/nav") + toc_start;
        let toc_block = &html[toc_start..toc_end];
        assert!(toc_block.contains("href=\"#a\""), "toc: {toc_block}");
        assert!(!toc_block.contains("href=\"#b\""), "toc: {toc_block}");
        assert!(!toc_block.contains("href=\"#c\""), "toc: {toc_block}");
    }

    #[test]
    fn default_css_inlined_unless_suppressed() {
        let opts = DocOptions::default();
        let html = markdown_to_html(b"hello\n", &opts).unwrap();
        assert!(html.contains("@page"), "html: {html}");

        let mut opts2 = DocOptions::default();
        opts2.no_default_css = true;
        let html2 = markdown_to_html(b"hello\n", &opts2).unwrap();
        assert!(!html2.contains("@page"), "html: {html2}");
    }

    #[test]
    fn gfm_enables_tables() {
        let mut opts = DocOptions::default();
        opts.gfm = true;
        let md = b"| h1 | h2 |\n|----|----|\n| a | b |\n";
        let html = markdown_to_html(md, &opts).unwrap();
        assert!(html.contains("<table"), "html: {html}");
    }

    #[test]
    fn slugify_strips_punctuation() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Rust 2024!"), "rust-2024");
        assert_eq!(slugify("  spaces  everywhere  "), "spaces-everywhere");
        assert_eq!(slugify("A/B test"), "ab-test");
    }

    #[test]
    fn custom_css_appended() {
        let mut opts = DocOptions::default();
        opts.custom_css = Some("body { color: red; }".into());
        let html = markdown_to_html(b"x\n", &opts).unwrap();
        assert!(html.contains("color: red"), "html: {html}");
    }
}
