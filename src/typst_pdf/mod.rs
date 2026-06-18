//! Embedded typst-based PDF engine (pure Rust, no external process).
//!
//! Task 1.1 scope: prove that a hardcoded typst source string can be compiled
//! to PDF bytes in-process via a minimal [`ReconWorld`]. The Markdown→typst
//! translator and CLI wiring arrive in later tasks.

mod images;
mod preamble;
mod translate;
mod world;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use comrak::nodes::AstNode;
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::docs::DocOptions;
use world::ReconWorld;

/// Render a parsed Markdown document (comrak AST) to PDF bytes via typst.
///
/// `base_dir` resolves relative local image paths (the markdown file's parent,
/// or the current directory for stdin); `http` fetches remote images. Image
/// bytes are inlined directly into the generated typst source, so the world's
/// file map stays empty.
pub fn render_md_to_pdf<'a>(
    root: &'a AstNode<'a>,
    opts: &DocOptions,
    base_dir: &Path,
    http: &reqwest::blocking::Client,
) -> Result<Vec<u8>> {
    // Build the cover (if requested): explicit template overrides the
    // metadata-driven auto cover.
    let cover = if let Some(tpl) = &opts.cover_template {
        let body = std::fs::read_to_string(tpl)
            .map_err(|e| anyhow!("--cover-template: read {}: {e}", tpl.display()))?;
        preamble::cover_from_template(
            &body,
            opts.title.as_deref().unwrap_or(""),
            opts.subtitle.as_deref(),
            opts.author.as_deref(),
            opts.version.as_deref(),
            opts.date.as_deref(),
        )
    } else if opts.cover {
        preamble::auto_cover(
            opts.title.as_deref().unwrap_or(""),
            opts.subtitle.as_deref(),
            opts.author.as_deref(),
            opts.version.as_deref(),
            opts.date.as_deref(),
        )
    } else {
        String::new()
    };

    let body = translate::body(root, opts, base_dir, http)?;
    let src = assemble(opts, &cover, &body)?;
    compile_to_pdf(src)
}

/// Sentinel emitted by the translator wherever a `<!-- toc -->` directive
/// appeared; replaced in-place by the outline (or stripped if no `--toc`).
const TOC_SENTINEL: &str = "%RECON_TOC%";

/// Assemble the full typst source in book order: preamble, optional
/// `--page-break-on-h1` show-rule, unnumbered front matter (cover + ToC),
/// then arabic-numbered body with a centered page-number footer.
///
/// The outline replaces an in-body `%RECON_TOC%` sentinel when present;
/// otherwise it is emitted between the cover and the body. With no `--toc`
/// any stray sentinel is stripped.
pub fn assemble(opts: &DocOptions, cover: &str, body: &str) -> Result<String> {
    use preamble::typ_str;

    let toc_enabled = opts.toc;
    let toc_depth = if opts.toc_depth == 0 { 3 } else { opts.toc_depth };
    let toc_title = opts.toc_title.as_str();
    let page_numbers = opts.page_numbers;
    let h1_breaks = opts.page_break_on_h1;

    let mut src = preamble::build_preamble(opts)?;

    if h1_breaks {
        src.push_str("#show heading.where(level: 1): it => pagebreak(weak: true) + it\n");
    }

    // Front matter: no page numbering for cover / ToC.
    src.push_str("#set page(numbering: none)\n");

    let outline = if toc_enabled {
        format!(
            "#outline(title: {}, depth: {})\n#pagebreak()\n",
            typ_str(toc_title),
            toc_depth
        )
    } else {
        String::new()
    };

    if !cover.is_empty() {
        src.push_str(cover);
    }

    // Body: drop the sentinel / substitute the outline.
    let body_assembled = if body.contains(TOC_SENTINEL) {
        if toc_enabled {
            body.replacen(TOC_SENTINEL, outline.trim_end(), 1)
        } else {
            // Strip the stray sentinel (and a trailing blank line if any).
            body.replacen(TOC_SENTINEL, "", 1)
        }
    } else {
        // No in-body sentinel: emit the outline here (after the cover).
        src.push_str(&outline);
        body.to_string()
    };

    // Body start: switch to arabic numbering with a centered footer.
    if page_numbers {
        src.push_str("#counter(page).update(1)\n");
        src.push_str(
            "#set page(numbering: \"1\", footer: context align(center)[#counter(page).display()])\n",
        );
    }

    src.push_str(&body_assembled);
    src.push('\n');
    Ok(src)
}

/// Compile a complete typst source string to PDF bytes.
///
/// Both the compile and PDF-export diagnostics are collapsed into a single
/// joined error message on failure.
pub fn compile_to_pdf(source: String) -> Result<Vec<u8>> {
    let world = ReconWorld::new(source, HashMap::new());

    // `typst::compile` is generic over the document type; the PDF backend
    // consumes a `PagedDocument`.
    let compiled = typst::compile::<PagedDocument>(&world);
    let document = compiled.output.map_err(|diags| {
        anyhow!(
            "typst compilation failed: {}",
            diags
                .iter()
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;

    typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|diags| {
        anyhow!(
            "typst PDF export failed: {}",
            diags
                .iter()
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_minimal_document_to_pdf() {
        let pdf = compile_to_pdf("#set page(paper: \"a4\")\n= Hello\n\nBody.".into()).unwrap();
        assert!(pdf.starts_with(b"%PDF-"), "not a PDF");
        assert!(pdf.len() > 500, "suspiciously small PDF");
    }

    #[test]
    fn assemble_orders_cover_outline_body() {
        let mut opts = DocOptions::default();
        opts.page_size = "a4".into();
        opts.toc = true;
        opts.toc_depth = 3;
        opts.toc_title = "Contents".into();
        opts.cover = true;
        opts.page_numbers = true;
        let src = assemble(&opts, "COVER_MARKER\n#pagebreak()\n", "BODY_MARKER\n").unwrap();

        let cover_i = src.find("COVER_MARKER").expect("cover present");
        let outline_i = src.find("#outline(").expect("outline present");
        let body_i = src.find("BODY_MARKER").expect("body present");
        assert!(cover_i < outline_i, "cover before outline");
        assert!(outline_i < body_i, "outline before body");

        // Numbering directives present.
        assert!(src.contains("#set page(numbering: none)"));
        assert!(src.contains("#counter(page).update(1)"));
        assert!(src.contains("numbering: \"1\""));
        assert!(src.contains("counter(page).display()"));
    }

    #[test]
    fn assemble_replaces_sentinel_in_place() {
        let mut opts = DocOptions::default();
        opts.page_size = "a4".into();
        opts.toc = true;
        opts.page_numbers = true;
        let src = assemble(&opts, "", "Intro\n\n%RECON_TOC%\n\nMore body\n").unwrap();
        assert!(!src.contains("%RECON_TOC%"), "sentinel should be gone");
        let outline_i = src.find("#outline(").expect("outline present");
        let intro_i = src.find("Intro").unwrap();
        let more_i = src.find("More body").unwrap();
        assert!(intro_i < outline_i && outline_i < more_i, "outline in place");
    }

    #[test]
    fn assemble_strips_sentinel_without_toc() {
        let opts = {
            let mut o = DocOptions::default();
            o.page_size = "a4".into();
            o.page_numbers = true;
            o
        };
        let src = assemble(&opts, "", "A\n\n%RECON_TOC%\n\nB\n").unwrap();
        assert!(!src.contains("%RECON_TOC%"));
        assert!(!src.contains("#outline("));
    }

    #[test]
    fn assemble_no_footer_when_page_numbers_off() {
        let mut opts = DocOptions::default();
        opts.page_size = "a4".into();
        opts.page_numbers = false;
        let src = assemble(&opts, "", "Body\n").unwrap();
        assert!(!src.contains("footer:"), "no footer expected");
        assert!(!src.contains("counter(page).update"), "no arabic switch expected");
    }
}
