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

/// A markdown image whose source is an inline `data:` PNG URI must embed in the
/// PDF — no network needed. We prove embedding by checking the image-bearing
/// PDF is larger than an otherwise-identical baseline with no image.
#[test]
fn typst_embeds_data_uri_png_image() {
    // A minimal valid 1x1 PNG (the same bytes the unit tests decode).
    const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";

    let dir = std::env::temp_dir();

    // Baseline: same text, no image.
    let base_md = dir.join("t_img_base.md");
    let base_pdf = dir.join("t_img_base.pdf");
    std::fs::write(&base_md, "# Pic\n\nSome text here.\n").unwrap();
    let out = Command::new(recon())
        .args(["--md-to-pdf", base_md.to_str().unwrap(), "-o", base_pdf.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "baseline stderr: {}", String::from_utf8_lossy(&out.stderr));

    // With an embedded data-URI PNG image.
    let img_md = dir.join("t_img_data.md");
    let img_pdf = dir.join("t_img_data.pdf");
    let md = format!("# Pic\n\nSome text here.\n\n![a dot](data:image/png;base64,{PNG_B64})\n");
    std::fs::write(&img_md, md).unwrap();
    let out = Command::new(recon())
        .args(["--md-to-pdf", img_md.to_str().unwrap(), "-o", img_pdf.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "image stderr: {}", String::from_utf8_lossy(&out.stderr));

    let base_len = std::fs::metadata(&base_pdf).unwrap().len();
    let img_len = std::fs::metadata(&img_pdf).unwrap().len();
    assert!(
        img_len > base_len,
        "expected image PDF ({img_len} bytes) larger than baseline ({base_len} bytes) — image not embedded"
    );
}

#[test]
fn full_gfm_roundtrip_preserves_literal_angle_brackets() {
    let dir = std::env::temp_dir();
    let md = dir.join("t_gfm.md");
    let pdf = dir.join("t_gfm.pdf");
    let src = "\
# Guide

A paragraph with **bold**, *italic*, ~~struck~~, and a [link](https://example.com).

| Command | Effect |
|---------|--------|
| init    | start  |
| commit  | save   |

```sh
git log <branch>
git commit <pathspec>
```

- top
  - nested
- [ ] todo
- [x] done

A footnote ref.[^n]

[^n]: the note text.
";
    std::fs::write(&md, src).unwrap();
    let out = std::process::Command::new(recon())
        .args(["--md-to-pdf", md.to_str().unwrap(), "-o", pdf.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));

    let txt = std::process::Command::new("pdftotext").arg(&pdf).arg("-").output().unwrap();
    let text = String::from_utf8_lossy(&txt.stdout);
    // literal <branch>/<pathspec> survive verbatim (R5)
    assert!(text.contains("<branch>"), "literal <branch> missing: {text}");
    assert!(text.contains("<pathspec>"), "literal <pathspec> missing: {text}");
    // table + footnote content present
    assert!(text.contains("commit") && text.contains("save"), "table text missing: {text}");
    assert!(text.contains("the note text"), "footnote text missing: {text}");
}

#[test]
fn typst_cover_and_toc_produces_multipage_pdf() {
    let dir = std::env::temp_dir();
    let md = dir.join("t_cover.md");
    let pdf = dir.join("t_cover.pdf");
    std::fs::write(
        &md,
        "# Chapter One\n\nBody one.\n\n# Chapter Two\n\nBody two.\n",
    )
    .unwrap();
    let out = Command::new(recon())
        .args([
            "--md-to-pdf",
            md.to_str().unwrap(),
            "--cover",
            "--toc",
            "--doc-title",
            "T",
            "-o",
            pdf.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let info = Command::new("pdfinfo").arg(&pdf).output().unwrap();
    assert!(info.status.success(), "pdfinfo failed");
    let s = String::from_utf8_lossy(&info.stdout);
    let pages: usize = s
        .lines()
        .find_map(|l| l.strip_prefix("Pages:"))
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(0);
    assert!(pages >= 2, "expected >=2 pages (cover + toc + body), got {pages}: {s}");
}

#[test]
fn typst_cover_template_renders_title() {
    let dir = std::env::temp_dir();
    let md = dir.join("t_covtpl.md");
    let pdf = dir.join("t_covtpl.pdf");
    let tpl = dir.join("t_cov_template.typ");
    std::fs::write(&md, "# Body Head\n\nSome content.\n").unwrap();
    std::fs::write(&tpl, "#align(center, text(20pt)[#title])\n").unwrap();
    let out = Command::new(recon())
        .args([
            "--md-to-pdf",
            md.to_str().unwrap(),
            "--cover-template",
            tpl.to_str().unwrap(),
            "--doc-title",
            "MyCoverTitle",
            "-o",
            pdf.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let txt = Command::new("pdftotext").arg(&pdf).arg("-").output().unwrap();
    let text = String::from_utf8_lossy(&txt.stdout);
    assert!(text.contains("MyCoverTitle"), "cover title missing: {text}");
}
