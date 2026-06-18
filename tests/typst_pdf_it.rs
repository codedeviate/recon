use std::process::Command;

fn recon() -> &'static str {
    env!("CARGO_BIN_EXE_recon")
}

#[test]
fn typst_default_produces_a4_pdf() {
    let dir = std::env::temp_dir();
    let md = dir.join("t_s1.md");
    let pdf = dir.join("t_s1.pdf");
    std::fs::write(&md, "# Title\n\nHello world.\n").unwrap();
    let out = Command::new(recon())
        .args(["--md-to-pdf", md.to_str().unwrap(), "-o", pdf.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let info = Command::new("pdfinfo").arg(&pdf).output().unwrap();
    let s = String::from_utf8_lossy(&info.stdout);
    // A4 = 595.276 x 841.89 pts; pdfinfo labels it "(A4)".
    assert!(s.contains("595") && s.contains("841"), "expected A4, got: {s}");
}
