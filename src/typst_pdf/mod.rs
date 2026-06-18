//! Embedded typst-based PDF engine (pure Rust, no external process).
//!
//! Task 1.1 scope: prove that a hardcoded typst source string can be compiled
//! to PDF bytes in-process via a minimal [`ReconWorld`]. The Markdown→typst
//! translator and CLI wiring arrive in later tasks.

mod preamble;
mod world;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use world::ReconWorld;

/// Compile a complete typst source string to PDF bytes.
///
/// Both the compile and PDF-export diagnostics are collapsed into a single
/// joined error message on failure.
// `dead_code`: the CLI/translator that calls this lands in a later task.
#[allow(dead_code)]
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
}
