//! HTML → PDF by shelling out to agent-browser.
//!
//! Flow: write the HTML to a tempfile → `agent-browser open file://…`
//! → `agent-browser pdf <output>` → `agent-browser close`. The temp
//! is dropped at end-of-scope. Close is attempted on error paths so
//! agent-browser doesn't leave a hung browser session behind.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Convert an HTML document to a PDF at `output`. Requires
/// agent-browser (and, through it, Chrome / Chromium). Returns a
/// clear error with install guidance if agent-browser is missing.
pub fn render_html_to_pdf(html: &[u8], output: &Path) -> Result<()> {
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
    Ok(())
}
