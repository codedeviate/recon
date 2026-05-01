//! HTML → PDF by shelling out to agent-browser.
//!
//! Flow: write the HTML to a tempfile → `agent-browser open file://…`
//! → `agent-browser pdf <output>` → `agent-browser close`. The temp
//! is dropped at end-of-scope. Close is attempted on error paths so
//! agent-browser doesn't leave a hung browser session behind.
//!
//! After PDF generation, `patch_pdf_info` post-processes the PDF bytes
//! to inject Author / Subject / Keywords into the existing Info
//! dictionary. Chrome's printToPDF does not read `<meta name="author">`
//! etc., so metadata must be written directly into the PDF.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// PDF metadata fields we can inject post-generation.
#[derive(Debug, Default)]
pub struct PdfMeta {
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
}

/// PDF-escape a string: backslash and parentheses need escaping inside
/// literal PDF strings `(...)`.
fn pdf_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// Post-process a PDF file to add Author / Subject / Keywords into the
/// Info dictionary. Chrome-generated PDFs always contain `1 0 obj` as
/// the Info object (verified empirically). The function:
///
/// 1. Reads the existing PDF bytes.
/// 2. Finds the `1 0 obj` … `endobj` block (the Info dict).
/// 3. Inserts the new fields before the closing `>>`.
/// 4. Rewrites every xref-table entry for objects whose byte offset
///    was after the insertion point (they all shift by the injection
///    length), then updates `startxref` to point at the new xref.
///
/// The traditional cross-reference format used by Chrome 147 contains
/// 20-byte fixed-width entries: `NNNNNNNNNN GGGGG f|n \n`.  We find the
/// xref table via `startxref`, parse it in-place, bump affected offsets,
/// and rewrite everything in one shot.
///
/// If any step fails we leave the PDF untouched — metadata is
/// best-effort and should never abort the pipeline.
fn patch_pdf_info(path: &Path, meta: &PdfMeta) -> Result<()> {
    if meta.author.is_none() && meta.subject.is_none() && meta.keywords.is_none() {
        return Ok(());
    }

    let original = std::fs::read(path).context("patch_pdf_info: read")?;

    // ── 1. Build the injection string ────────────────────────────────
    let mut injection = String::new();
    if let Some(a) = &meta.author {
        injection.push_str(&format!("/Author ({})\n", pdf_escape(a)));
    }
    if let Some(s) = &meta.subject {
        injection.push_str(&format!("/Subject ({})\n", pdf_escape(s)));
    }
    if let Some(k) = &meta.keywords {
        injection.push_str(&format!("/Keywords ({})\n", pdf_escape(k)));
    }
    let delta = injection.len(); // how many bytes we're inserting

    // ── 2. Find insertion point inside `1 0 obj` ─────────────────────
    let marker = b"1 0 obj\n<<";
    let start = original
        .windows(marker.len())
        .position(|w| w == marker)
        .context("patch_pdf_info: Info object not found")?;

    let endobj_marker = b"\nendobj";
    let endobj_pos = original[start..]
        .windows(endobj_marker.len())
        .position(|w| w == endobj_marker)
        .map(|p| start + p)
        .context("patch_pdf_info: endobj not found")?;

    // Last `>>` before endobj — the closing delimiter of the Info dict.
    let info_slice = &original[start..endobj_pos];
    let closing_rel = info_slice
        .windows(2)
        .rposition(|w| w == b">>")
        .context("patch_pdf_info: closing >> not found in Info dict")?;
    let insert_pos = start + closing_rel;

    // ── 3. Build patched bytes with injection inserted ────────────────
    let mut patched = Vec::with_capacity(original.len() + delta);
    patched.extend_from_slice(&original[..insert_pos]);
    patched.extend_from_slice(injection.as_bytes());
    patched.extend_from_slice(&original[insert_pos..]);

    // ── 4. Find and patch the xref table ─────────────────────────────
    // Locate startxref → get old xref table offset.
    let startxref_tag = b"startxref\n";
    let sxref_pos = patched
        .windows(startxref_tag.len())
        .rposition(|w| w == startxref_tag)
        .context("patch_pdf_info: startxref not found")?;

    let after_sxref = sxref_pos + startxref_tag.len();
    let newline_in_sxref = patched[after_sxref..]
        .iter()
        .position(|&b| b == b'\n')
        .context("patch_pdf_info: no newline after startxref")?;
    let old_xref_offset: usize = std::str::from_utf8(
        &patched[after_sxref..after_sxref + newline_in_sxref],
    )
    .ok()
    .and_then(|s| s.trim().parse().ok())
    .context("patch_pdf_info: could not parse xref offset")?;

    // The xref table in the patched bytes is shifted by delta (because
    // the insertion was before it).
    let new_xref_offset = old_xref_offset + delta;

    // Locate the xref table itself in patched and update each entry
    // whose object offset > insert_pos.  Traditional xref entries are
    // exactly 20 bytes: "NNNNNNNNNN GGGGG f \n" (the last char is
    // space then \n, or \r\n on some writers — we handle both).
    let xref_tag = b"xref\n";
    // The xref table is at new_xref_offset in patched (already shifted).
    let xref_start = new_xref_offset;
    if xref_start + xref_tag.len() > patched.len()
        || &patched[xref_start..xref_start + xref_tag.len()] != xref_tag
    {
        anyhow::bail!("patch_pdf_info: xref table not found at expected offset");
    }

    // Parse the subsection header "0 N\n"
    let after_xref_tag = xref_start + xref_tag.len();
    let header_newline = patched[after_xref_tag..]
        .iter()
        .position(|&b| b == b'\n')
        .context("patch_pdf_info: no newline after xref header")?;
    let header = std::str::from_utf8(&patched[after_xref_tag..after_xref_tag + header_newline])
        .context("patch_pdf_info: xref header not UTF-8")?;
    let obj_count: usize = header
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .context("patch_pdf_info: could not parse xref object count")?;

    let entries_start = after_xref_tag + header_newline + 1;

    // Each entry is 20 bytes.  Bump offsets of objects after insert_pos.
    for i in 0..obj_count {
        let entry_pos = entries_start + i * 20;
        if entry_pos + 20 > patched.len() {
            break;
        }
        // Entry flag is byte 17 ('f' = free, 'n' = in-use).
        if patched[entry_pos + 17] != b'n' {
            continue;
        }
        let offset_bytes = &patched[entry_pos..entry_pos + 10];
        let offset: usize = std::str::from_utf8(offset_bytes)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        // Object 1 itself starts at insert_pos origin; it doesn't move
        // because our injection is inside it.  All objects after the
        // inserted bytes need their offsets bumped.
        if offset > insert_pos {
            let new_offset = offset + delta;
            let new_str = format!("{:010}", new_offset);
            patched[entry_pos..entry_pos + 10].copy_from_slice(new_str.as_bytes());
        }
    }

    // ── 5. Update startxref to point at new xref position ────────────
    let new_xref_str = new_xref_offset.to_string();
    // Rebuild the tail so the length changes don't invalidate our
    // position math (the new_xref_str might be longer/shorter than the
    // old value — in practice same length, but be safe).
    let tail_before = &patched[..after_sxref];
    let tail_after_newline = after_sxref + newline_in_sxref; // points at '\n'
    let tail_rest = &patched[tail_after_newline..];
    let mut final_bytes = Vec::with_capacity(patched.len());
    final_bytes.extend_from_slice(tail_before);
    final_bytes.extend_from_slice(new_xref_str.as_bytes());
    final_bytes.extend_from_slice(tail_rest);

    std::fs::write(path, &final_bytes).context("patch_pdf_info: write")?;
    Ok(())
}

/// Convert an HTML document to a PDF at `output`. Requires
/// agent-browser (and, through it, Chrome / Chromium). Returns a
/// clear error with install guidance if agent-browser is missing.
pub fn render_html_to_pdf(html: &[u8], output: &Path) -> Result<()> {
    render_html_to_pdf_with_meta(html, output, &PdfMeta::default())
}

/// Convert an HTML document to a PDF at `output`, then inject the
/// given metadata fields (author / subject / keywords) into the PDF's
/// Info dictionary.
pub fn render_html_to_pdf_with_meta(html: &[u8], output: &Path, meta: &PdfMeta) -> Result<()> {
    if !crate::agent_browser::state_snapshot().available {
        anyhow::bail!(
            "HTML→PDF needs agent-browser (which wraps Chrome's printToPDF). \
             Install via `brew install agent-browser` or \
             `npm install -g agent-browser` and retry."
        );
    }

    let mut tmp = tempfile::Builder::new()
        .prefix("recon-doc-")
        .suffix(".html")
        .tempfile()
        .context("docs_pdf: create tempfile")?;
    tmp.write_all(html).context("docs_pdf: write tempfile")?;
    tmp.flush().ok();

    let abs_tmp = tmp
        .path()
        .canonicalize()
        .context("docs_pdf: canonicalize tempfile path")?;
    let url = format!("file://{}", abs_tmp.display());

    let open_result =
        crate::agent_browser::run_cmd(&["open", &url], false).map(|_| ());
    let pdf_result = if open_result.is_ok() {
        let out_str = output
            .to_str()
            .context("docs_pdf: output path is not UTF-8")?;
        crate::agent_browser::run_cmd(&["pdf", out_str], false).map(|_| ())
    } else {
        Err(open_result.unwrap_err())
    };

    // Always attempt a close so agent-browser doesn't leak a session.
    let _ = crate::agent_browser::run_cmd(&["close"], false);

    pdf_result.context("docs_pdf: agent-browser pdf failed")?;

    // Post-process: inject author / subject / keywords into the PDF Info dict.
    // Failures here are best-effort — we log a warning but don't abort.
    if meta.author.is_some() || meta.subject.is_some() || meta.keywords.is_some() {
        if let Err(e) = patch_pdf_info(output, meta) {
            eprintln!("recon: warning: could not patch PDF metadata: {e}");
        }
    }

    Ok(())
}
