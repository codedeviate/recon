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
    white-space: pre-wrap;
    overflow-wrap: break-word;
    page-break-inside: avoid;
}
code { background: #f5f5f5; padding: 1px 4px; border-radius: 3px; }
pre code { background: transparent; padding: 0; }
pre code .c { color: #6a737d; }
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

/* --- Page-break helpers (need --unsafe-html to be set in markdown) --- */

/* Explicit break marker: put <div class="page-break"></div> in markdown. */
div.page-break { break-after: page; page-break-after: always; height: 0; }

/* Inline marker equivalent for compatibility: <hr class="page-break"> */
hr.page-break { border: 0; break-after: page; page-break-after: always; }

/* --- Cover page (<div class="cover">…</div> in markdown with --unsafe-html) --- */

div.cover {
    min-height: 90vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 2em 1em;
    break-after: page;
    page-break-after: always;
}
div.cover h1 {
    font-size: 48pt;
    border: 0;
    margin: 0 0 0.2em 0;
    font-weight: 600;
    color: #111;
}
div.cover .subtitle {
    font-size: 20pt;
    color: #555;
    margin-bottom: 2em;
}
div.cover .version,
div.cover .date,
div.cover .author,
div.cover .meta {
    font-size: 12pt;
    color: #666;
    margin: 0.2em 0;
    font-family: "SF Mono", Menlo, Consolas, monospace;
}
div.cover hr {
    width: 60%;
    margin: 1.5em auto;
    border: 0;
    border-top: 1px solid #ccc;
}
"#;

/// Options for document conversion. Parsed from CLI flags or opts map.
#[derive(Debug, Clone, Default)]
pub struct DocOptions {
    pub toc: bool,
    pub toc_depth: u8,
    pub toc_title: String,
    /// Strip inline formatting (code/bold/italic) from TOC entries.
    /// The HTML / chrome path is always plain; this only changes the
    /// typst outline, which otherwise mirrors heading formatting.
    pub toc_plain: bool,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub custom_css: Option<String>,
    pub no_default_css: bool,
    pub gfm: bool,
    pub unsafe_html: bool,
    pub page_break_on_h1: bool,
    pub pdf_engine: crate::cli::PdfEngine,
    pub page_size: String,
    pub cover: bool,
    pub cover_template: Option<std::path::PathBuf>,
    pub subtitle: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
    pub page_numbers: bool,
    /// Body text font (typst engine). `None` keeps the default serif
    /// (Libertinus Serif). Must name a bundled or `--font-path` font.
    pub font: Option<String>,
    /// Extra font directories the typst engine scans (`--font-path`),
    /// so `--font` can resolve user/system fonts. Empty = bundled only.
    pub font_path: Vec<String>,
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
            // Plain by default; --no-toc-plain opts back into formatted
            // outline entries. `overrides_with` makes the last of
            // --toc-plain / --no-toc-plain on the command line win.
            toc_plain: !args.no_toc_plain,
            title: args.doc_title.clone(),
            author: args.doc_author.clone(),
            subject: args.doc_subject.clone(),
            keywords: args.doc_keywords.clone(),
            custom_css,
            no_default_css: args.no_default_css,
            gfm: args.gfm,
            unsafe_html: args.unsafe_html,
            page_break_on_h1: args.page_break_on_h1,
            pdf_engine: args.pdf_engine,
            page_size: args.page_size.clone(),
            cover: args.cover || args.cover_template.is_some(),
            cover_template: args.cover_template.clone(),
            subtitle: args.doc_subtitle.clone(),
            version: args.doc_version.clone(),
            date: args.doc_date.clone(),
            page_numbers: !args.no_page_numbers,
            font: args.font.clone(),
            font_path: args.font_path.clone(),
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
    let mut body_str = String::from_utf8(body).context("comrak output is not UTF-8")?;

    // If the author placed a `<!-- toc -->` marker in the markdown,
    // expand it in-place and suppress the automatic top-of-body
    // injection — lets users position the TOC after a cover page.
    // The marker survives the comrak pipeline only when
    // `--unsafe-html` is on.
    let toc_in_body = body_str.contains("<!-- toc -->");
    if toc_in_body {
        body_str = body_str.replacen("<!-- toc -->", &toc_html, 1);
    }
    let top_toc = if toc_in_body { "" } else { toc_html.as_str() };

    let title = opts.title.clone().unwrap_or_else(|| "Document".into());
    Ok(wrap_document(&title, opts, top_toc, &body_str))
}

pub fn comrak_options(opts: &DocOptions) -> Options<'_> {
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
    render.unsafe_ = opts.unsafe_html;

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

fn wrap_document(title: &str, opts: &DocOptions, top_toc_html: &str, body_html: &str) -> String {
    let mut css = String::new();
    if !opts.no_default_css {
        css.push_str(DEFAULT_CSS);
    }
    if opts.page_break_on_h1 {
        // Every top-level heading after the first starts a new page.
        // Skip the cover block's own <h1> (it handles its own break).
        css.push_str(
            "\n/* --page-break-on-h1 */\n\
             main > h1:not(:first-of-type) {\n\
             \x20\x20break-before: page;\n\
             \x20\x20page-break-before: always;\n\
             }\n",
        );
    }
    if let Some(extra) = opts.custom_css.as_deref() {
        css.push('\n');
        css.push_str(extra);
    }

    let mut meta_tags = String::new();
    if let Some(author) = opts.author.as_deref() {
        meta_tags.push_str(&format!(
            "  <meta name=\"author\" content=\"{}\">\n",
            html_escape(author)
        ));
    }
    if let Some(subject) = opts.subject.as_deref() {
        meta_tags.push_str(&format!(
            "  <meta name=\"description\" content=\"{}\">\n",
            html_escape(subject)
        ));
    }
    if let Some(keywords) = opts.keywords.as_deref() {
        meta_tags.push_str(&format!(
            "  <meta name=\"keywords\" content=\"{}\">\n",
            html_escape(keywords)
        ));
    }

    format!(
        "<!doctype html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <title>{title}</title>\n\
{meta_tags}\
  <style>\n{css}\n  </style>\n\
</head>\n\
<body>\n\
<main>\n\
{toc}{body}\n\
</main>\n\
<script>\n\
(function(){{\n\
  function esc(s){{return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}}\n\
  document.querySelectorAll('pre code').forEach(function(block){{\n\
    var lines=block.textContent.split('\\n');\n\
    var html=lines.map(function(line){{\n\
      var m=line.match(/^(\\s*)(#(?!\\{{).*)$/);\n\
      if(m) return esc(m[1])+'<span class=\"c\">'+esc(m[2])+'</span>';\n\
      var ci=line.indexOf(' # ');\n\
      if(ci!==-1) return esc(line.slice(0,ci))+'<span class=\"c\">'+esc(line.slice(ci))+'</span>';\n\
      return esc(line);\n\
    }}).join('\\n');\n\
    block.innerHTML=html;\n\
  }});\n\
}})();\n\
</script>\n\
</body>\n\
</html>\n",
        title = html_escape(title),
        meta_tags = meta_tags,
        css = css,
        toc = top_toc_html,
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

    // Engine-specific flag validation, before any rendering work.
    match opts.pdf_engine {
        crate::cli::PdfEngine::Typst => {
            // The typst engine uses its own styling; the CSS/raw-HTML flags only
            // apply to the chrome (HTML) path. (Raw-HTML-in-markdown is rejected
            // separately by the typst translator.)
            if opts.custom_css.is_some() {
                anyhow::bail!(
                    "--doc-css is not supported by the typst engine (it uses its own styling). \
                     Re-run with --pdf-engine chrome."
                );
            }
            if opts.no_default_css {
                anyhow::bail!(
                    "--no-default-css is not supported by the typst engine (it uses its own \
                     styling). Re-run with --pdf-engine chrome."
                );
            }
            if opts.unsafe_html {
                anyhow::bail!(
                    "--unsafe-html is not supported by the typst engine (it uses its own \
                     styling). Re-run with --pdf-engine chrome."
                );
            }
        }
        crate::cli::PdfEngine::Chrome => {
            // --page-size defaults to "a4" via clap, so we cannot distinguish an
            // explicit `--page-size a4` from the unset default. The pragmatic rule
            // is to reject only a non-"a4" value, which is unambiguously user-set;
            // a bare "a4" is silently accepted (the chrome engine renders Letter).
            if opts.page_size.to_ascii_lowercase() != "a4" {
                anyhow::bail!(
                    "--page-size is only supported by the typst engine; the chrome engine renders \
                     US Letter. Drop --page-size or use the typst engine."
                );
            }
            if opts.font.is_some() {
                anyhow::bail!(
                    "--font is only supported by the typst engine; the chrome engine styles fonts \
                     via CSS (--doc-css). Drop --font or use the typst engine."
                );
            }
            if !opts.font_path.is_empty() {
                anyhow::bail!(
                    "--font-path is only supported by the typst engine. Drop --font-path or use \
                     the typst engine."
                );
            }
        }
    }

    let bytes = load_source(args, src)?;

    match opts.pdf_engine {
        crate::cli::PdfEngine::Typst => {
            if opts.subject.is_some() {
                eprintln!(
                    "recon: warning: --doc-subject is not supported by the typst engine; \
                     use --pdf-engine chrome for a Subject field"
                );
            }
            for dir in &opts.font_path {
                if !std::path::Path::new(dir).is_dir() {
                    eprintln!(
                        "recon: warning: --font-path '{dir}' is not a directory; ignoring"
                    );
                }
            }
            let source = std::str::from_utf8(&bytes).context("markdown is not valid UTF-8")?;
            let arena = Arena::new();
            let comrak_opts = comrak_options(&opts);
            let root = parse_document(&arena, source, &comrak_opts);

            // Base directory for resolving relative local image paths: the
            // markdown file's parent, or the current directory for stdin (`-`)
            // and remote/non-path sources.
            let base_dir = if src == "-" {
                std::env::current_dir().context("resolve current directory for stdin base")?
            } else {
                let p = std::path::Path::new(src);
                match p.parent() {
                    Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                    _ => std::env::current_dir()
                        .context("resolve current directory for image base")?,
                }
            };

            // HTTP client for remote (`http(s)`) image fetches: honour
            // `--insecure` and the connect timeout. A build failure here is
            // non-fatal per image (resolve returns Err → alt-text fallback),
            // but the client itself must construct.
            let http = reqwest::blocking::Client::builder()
                .danger_accept_invalid_certs(args.insecure)
                .connect_timeout(std::time::Duration::from_secs(args.timeout))
                .build()
                .unwrap_or_default();

            let pdf = crate::typst_pdf::render_md_to_pdf(root, &opts, &base_dir, &http)
                .context("typst: render markdown to PDF")?;
            std::fs::write(output, &pdf)
                .with_context(|| format!("write output '{}'", output.display()))
        }
        crate::cli::PdfEngine::Chrome => {
            let meta = crate::docs_pdf::PdfMeta {
                author: args.doc_author.clone(),
                subject: args.doc_subject.clone(),
                keywords: args.doc_keywords.clone(),
            };
            // Translate the HTML-free `<!-- page-break -->` directive into the
            // chrome path's existing `<div class="page-break">` mechanism so it
            // works on both engines without requiring --unsafe-html. comrak drops
            // raw HTML comments (and bare divs without --unsafe-html), so we swap
            // the comment for a sentinel token *before* comrak — the token survives
            // as paragraph text — then replace the rendered `<p>TOKEN</p>` with the
            // page-break div in the output HTML.
            const PB_TOKEN: &str = "RECONxPAGExBREAKxSENTINEL";
            let src_str = std::str::from_utf8(&bytes).context("markdown is not valid UTF-8")?;
            let preprocessed = src_str.replace("<!-- page-break -->", PB_TOKEN);
            let mut html = markdown_to_html(preprocessed.as_bytes(), &opts)?;
            html = html
                .replace(
                    &format!("<p>{PB_TOKEN}</p>"),
                    "<div class=\"page-break\"></div>",
                )
                .replace(PB_TOKEN, "<div class=\"page-break\"></div>");
            crate::docs_pdf::render_html_to_pdf_with_meta(html.as_bytes(), output, &meta)
        }
    }
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
    let meta = crate::docs_pdf::PdfMeta {
        author: args.doc_author.clone(),
        subject: args.doc_subject.clone(),
        keywords: args.doc_keywords.clone(),
    };
    let bytes = load_source(args, src)?;
    crate::docs_pdf::render_html_to_pdf_with_meta(&bytes, output, &meta)
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

    #[test]
    fn unsafe_html_passes_raw_markup_through() {
        let mut opts = DocOptions::default();
        opts.unsafe_html = true;
        let md = b"# T\n\n<div class=\"cover\"><h1>Hello</h1></div>\n\nBody.\n";
        let html = markdown_to_html(md, &opts).unwrap();
        assert!(
            html.contains("<div class=\"cover\"><h1>Hello</h1></div>"),
            "html: {html}",
        );
    }

    #[test]
    fn unsafe_html_off_escapes_raw_markup() {
        let opts = DocOptions::default();
        let md = b"# T\n\n<div class=\"cover\">Hello</div>\n\nBody.\n";
        let html = markdown_to_html(md, &opts).unwrap();
        // When unsafe_html is off, comrak replaces the raw block with
        // a comment or strips it — either way the class attribute
        // doesn't survive as live HTML.
        assert!(
            !html.contains("<div class=\"cover\">Hello</div>"),
            "expected raw div to be stripped, html: {html}",
        );
    }

    #[test]
    fn page_break_on_h1_injects_css_rule() {
        let mut opts = DocOptions::default();
        opts.page_break_on_h1 = true;
        let html = markdown_to_html(b"# A\n", &opts).unwrap();
        assert!(
            html.contains("break-before: page"),
            "expected break-before rule, html: {html}",
        );
        assert!(
            html.contains("main > h1:not(:first-of-type)"),
            "expected H1 selector, html: {html}",
        );
    }

    #[test]
    fn page_break_on_h1_disabled_omits_rule() {
        let opts = DocOptions::default();
        let html = markdown_to_html(b"# A\n", &opts).unwrap();
        assert!(
            !html.contains("--page-break-on-h1"),
            "expected no page-break CSS, html: {html}",
        );
    }
}
