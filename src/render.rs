//! HTML → text rendering (lynx/w3m-style text-browser view).
//!
//! Pure-Rust via the `html2text` crate (html5ever parser). No JavaScript,
//! no CSS layout, no images beyond `alt` text. One core function,
//! `render_html`, is reused by the `--html-to-text` CLI mode, the `--render`
//! response hook, and the `html_to_text(...)` script binding.

use anyhow::Context;
use anyhow::Result;
use std::io::IsTerminal;

use crate::cli::{Args, ColorWhen};

/// Per-call rendering options.
pub struct RenderOpts {
    /// Explicit wrap width; `None` resolves from the terminal (or 80).
    pub width: Option<usize>,
    /// When to emit ANSI styling.
    pub color: ColorWhen,
    /// Suppress link surfacing: in plain mode drop the `[N]` footnote
    /// markers and the trailing URL reference list (anchor text stays
    /// inline); in coloured mode drop the inline link styling.
    pub no_links: bool,
}

/// Resolve the wrap width: explicit override, else terminal columns on a
/// TTY, else 80. Floored at 20 so `--width 0` can never divide-by-zero.
fn resolve_width(opts: &RenderOpts) -> usize {
    let raw = match opts.width {
        Some(w) => w,
        None => {
            if std::io::stdout().is_terminal() {
                crossterm::terminal::size().map(|(c, _)| c as usize).unwrap_or(80)
            } else {
                80
            }
        }
    };
    raw.max(20)
}

/// Resolve whether ANSI styling should be emitted.
fn use_color(opts: &RenderOpts) -> bool {
    match opts.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => std::io::stdout().is_terminal(),
    }
}

/// True for content types recon will render as text.
pub fn is_html(content_type: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    ct.contains("text/html") || ct.contains("application/xhtml+xml")
}

/// Render an HTML string to text. The one true renderer.
pub fn render_html(html: &str, opts: &RenderOpts) -> Result<String> {
    let width = resolve_width(opts);
    if use_color(opts) {
        render_coloured(html, width, opts.no_links)
    } else {
        render_plain(html, width, opts.no_links)
    }
}

fn render_plain(html: &str, width: usize, no_links: bool) -> Result<String> {
    // link_footnotes(false) keeps anchor text inline but drops both the
    // `[N]` markers and the trailing URL reference list.
    html2text::config::plain()
        .link_footnotes(!no_links)
        .string_from_read(html.as_bytes(), width)
        .map_err(|e| anyhow::anyhow!("html render: {e}"))
}

fn render_coloured(html: &str, width: usize, no_links: bool) -> Result<String> {
    use html2text::render::RichAnnotation;

    html2text::from_read_coloured(html.as_bytes(), width, move |anns: &[RichAnnotation], text: &str| {
        let mut prefix = String::new();
        for ann in anns {
            match ann {
                RichAnnotation::Strong => prefix.push_str("\u{1b}[1m"),
                RichAnnotation::Emphasis => prefix.push_str("\u{1b}[3m"),
                RichAnnotation::Strikeout => prefix.push_str("\u{1b}[9m"),
                RichAnnotation::Code | RichAnnotation::Preformat(_) => prefix.push_str("\u{1b}[2m"),
                // Skip link styling when --render-no-links is set.
                RichAnnotation::Link(_) if !no_links => prefix.push_str("\u{1b}[4;34m"),
                _ => {}
            }
        }
        if prefix.is_empty() {
            text.to_string()
        } else {
            format!("{prefix}{text}\u{1b}[0m")
        }
    })
    .map_err(|e| anyhow::anyhow!("html render (coloured): {e}"))
}

/// CLI entry point for `--html-to-text`. Loads SRC (path / URL / `-`),
/// renders it, and writes to `-o PATH` or stdout.
pub fn run_html_to_text(args: &Args) -> Result<()> {
    use std::io::Write;

    let src = args
        .html_to_text
        .as_ref()
        .context("--html-to-text requires a source (path / URL / -)")?;

    // Reuse recon's universal source loader (file / http(s) / stdin).
    let mut call_args = args.clone();
    call_args.html_to_text = None;
    call_args.url = Some(src.to_string());
    call_args.url_flag = None;
    let bytes = crate::source::read_all(&call_args)?;

    let html = String::from_utf8_lossy(&bytes);
    let opts = RenderOpts { width: args.width, color: args.render_color, no_links: args.render_no_links };
    let text = render_html(&html, &opts)?;

    match args.output.as_ref() {
        Some(path) => std::fs::write(path, text.as_bytes())
            .with_context(|| format!("write output '{}'", path.display()))?,
        None => std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .context("write stdout")?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(html: &str) -> String {
        render_html(html, &RenderOpts { width: Some(60), color: ColorWhen::Never, no_links: false }).unwrap()
    }

    fn plain_no_links(html: &str) -> String {
        render_html(html, &RenderOpts { width: Some(60), color: ColorWhen::Never, no_links: true }).unwrap()
    }

    #[test]
    fn renders_heading_and_paragraph() {
        let out = plain("<h1>Title</h1><p>Hello world.</p>");
        assert!(out.contains("Title"), "out: {out:?}");
        assert!(out.contains("Hello world."), "out: {out:?}");
    }

    #[test]
    fn renders_unordered_list() {
        let out = plain("<ul><li>alpha</li><li>beta</li></ul>");
        assert!(out.contains("alpha"), "out: {out:?}");
        assert!(out.contains("beta"), "out: {out:?}");
    }

    #[test]
    fn links_become_footnote_references() {
        let out = plain(r#"<p>See <a href="https://example.com/x">the site</a>.</p>"#);
        assert!(out.contains("the site"), "anchor text missing: {out:?}");
        assert!(out.contains("https://example.com/x"), "url missing from footnotes: {out:?}");
    }

    #[test]
    fn decodes_entities() {
        let out = plain("<p>Tom &amp; Jerry</p>");
        assert!(out.contains("Tom & Jerry"), "out: {out:?}");
    }

    #[test]
    fn renders_table_cells() {
        let out = plain("<table><tr><td>r1c1</td><td>r1c2</td></tr></table>");
        assert!(out.contains("r1c1") && out.contains("r1c2"), "out: {out:?}");
    }

    #[test]
    fn empty_input_is_not_an_error() {
        let out = plain("   ");
        assert!(out.trim().is_empty(), "out: {out:?}");
    }

    #[test]
    fn wraps_to_explicit_width() {
        let long = "word ".repeat(40);
        let out = plain(&format!("<p>{long}</p>"));
        for line in out.lines() {
            // ASCII content only, so char count == display columns here.
            assert!(line.chars().count() <= 60, "line too long ({}): {line:?}", line.chars().count());
        }
    }

    #[test]
    fn explicit_width_overrides() {
        assert_eq!(resolve_width(&RenderOpts { width: Some(42), color: ColorWhen::Never, no_links: false }), 42);
    }

    #[test]
    fn zero_width_is_floored() {
        assert_eq!(resolve_width(&RenderOpts { width: Some(0), color: ColorWhen::Never, no_links: false }), 20);
    }

    #[test]
    fn is_html_recognises_html() {
        assert!(is_html("text/html; charset=utf-8"));
    }

    #[test]
    fn is_html_recognises_xhtml() {
        assert!(is_html("application/xhtml+xml"));
    }

    #[test]
    fn is_html_is_case_insensitive() {
        assert!(is_html("Text/HTML"));
    }

    #[test]
    fn is_html_rejects_non_html() {
        assert!(!is_html("application/json"));
        assert!(!is_html("text/plain"));
    }

    #[test]
    fn color_always_and_never_are_literal() {
        assert!(use_color(&RenderOpts { width: None, color: ColorWhen::Always, no_links: false }));
        assert!(!use_color(&RenderOpts { width: None, color: ColorWhen::Never, no_links: false }));
    }

    fn coloured(html: &str) -> String {
        render_html(html, &RenderOpts { width: Some(60), color: ColorWhen::Always, no_links: false }).unwrap()
    }

    fn coloured_no_links(html: &str) -> String {
        render_html(html, &RenderOpts { width: Some(60), color: ColorWhen::Always, no_links: true }).unwrap()
    }

    #[test]
    fn colour_mode_emits_ansi() {
        let out = coloured("<p><strong>bold</strong> and <em>italic</em></p>");
        assert!(out.contains('\u{1b}'), "expected ANSI escapes, got: {out:?}");
        assert!(out.contains("bold") && out.contains("italic"), "text missing: {out:?}");
    }

    #[test]
    fn plain_mode_emits_no_ansi() {
        let out = plain("<p><strong>bold</strong></p>");
        assert!(!out.contains('\u{1b}'), "unexpected ANSI escapes: {out:?}");
    }

    #[test]
    fn no_links_drops_footnote_list_but_keeps_anchor_text() {
        let html = r#"<p>See <a href="https://example.com/x">the site</a>.</p>"#;
        let out = plain_no_links(html);
        assert!(out.contains("the site"), "anchor text should remain: {out:?}");
        assert!(!out.contains("https://example.com/x"), "URL should be gone: {out:?}");
        assert!(!out.contains("[1]"), "footnote marker should be gone: {out:?}");
    }

    #[test]
    fn default_plain_keeps_url_footnotes() {
        // Regression guard: without --render-no-links the URL list stays.
        let out = plain(r#"<p>See <a href="https://example.com/x">the site</a>.</p>"#);
        assert!(out.contains("https://example.com/x"), "URL footnote expected: {out:?}");
    }

    #[test]
    fn no_links_coloured_drops_link_styling() {
        let html = r#"<p><a href="https://example.com/">link</a></p>"#;
        let styled = coloured(html);
        let unstyled = coloured_no_links(html);
        assert!(styled.contains("\u{1b}[4;34m"), "default should style links: {styled:?}");
        assert!(!unstyled.contains("\u{1b}[4;34m"), "no_links should drop link ANSI: {unstyled:?}");
        assert!(unstyled.contains("link"), "link text should remain: {unstyled:?}");
    }
}
