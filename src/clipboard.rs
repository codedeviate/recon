//! Cross-platform clipboard read/write via the `arboard` crate.
//!
//! Used by the `--from-clipboard` / `--to-clipboard` / `--clipboard` CLI
//! flags (in main.rs), the `BodySink::Clipboard` variant (in output.rs),
//! and the `clipboard::get` / `clipboard::set` Rhai bindings (in
//! script/bindings/clipboard.rs).

use anyhow::{Context, Result};

/// Read the current clipboard text. Returns Err if no clipboard is available
/// (no display server on Linux, sandboxed environment, etc.) or the clipboard
/// content can't be decoded as text.
pub fn read_text() -> Result<String> {
    let mut cb = arboard::Clipboard::new()
        .context("clipboard unavailable")?;
    cb.get_text().context("failed to read clipboard text")
}

/// Replace the clipboard contents with `text`.
pub fn write_text(text: &str) -> Result<()> {
    let mut cb = arboard::Clipboard::new()
        .context("clipboard unavailable")?;
    cb.set_text(text.to_string())
        .context("failed to write clipboard text")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip test, gated on env var because CI Linux usually has no display server.
    /// Run locally with: RECON_CLIPBOARD_TESTS=1 cargo test clipboard::
    #[test]
    fn round_trip() {
        if std::env::var_os("RECON_CLIPBOARD_TESTS").is_none() {
            return;
        }
        let original = read_text().ok();
        let test_payload = "recon clipboard test 2026-04-30";
        write_text(test_payload).expect("write should succeed");
        let read_back = read_text().expect("read should succeed");
        assert_eq!(read_back, test_payload);
        if let Some(orig) = original {
            let _ = write_text(&orig);
        }
    }
}
