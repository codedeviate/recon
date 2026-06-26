// doc-convert.rhai — md_to_html / md_to_pdf / html_to_pdf demo.
//
// Usage: recon --script doc-convert [MD_PATH] [PDF_OUT]
//
// 1. Read a markdown source (CLI arg or Cargo.toml as fallback demo).
// 2. Convert to HTML with a linkable TOC.
// 3. Optionally convert to PDF via agent-browser (skipped gracefully
//    when agent-browser isn't installed).

let src = if args.len() > 1 { args[1] } else { "CHANGELOG.md" };
let pdf_out = if args.len() > 2 { args[2] } else { "/tmp/recon-doc.pdf" };

// Load markdown. `file_read` returns a Blob; `Blob::to_string()` yields
// the hex-debug view, not a UTF-8 decoded String — `md_to_html` would
// silently render the hex bytes as a giant paragraph. Use `text::decode`
// to get the real text body.
let md = text::decode(file_read(src), "utf-8");

// 1. Markdown → HTML with TOC + GFM.
let html = md_to_html(md, #{ toc: true, toc_depth: 3, gfm: true, title: src });
print(`md → html: ${html.len()} bytes`);

// 2. Try HTML → PDF.
try {
    html_to_pdf(html, pdf_out);
    print(`html → pdf: wrote ${pdf_out}`);
} catch(e) {
    print(`html → pdf: skipped (${e})`);
}

// Typst engine with cover page and metadata (requires no agent-browser):
// md_to_pdf(md, pdf_out, #{ pdf_engine: "typst", page_size: "a4", cover: true,
//     doc_title: "My Book", doc_subtitle: "A subtitle", doc_version: "1.0",
//     doc_date: "2026-06-18", toc: true });

// Typst engine with a sans-serif body font (bundled IBM Plex Sans). Code
// stays on the monospace font. `font_path` (string or array) adds extra
// font directories so `font` can resolve user/system fonts.
let sans_out = "/tmp/recon-doc-sans.pdf";
try {
    md_to_pdf(md, sans_out, #{ font: "IBM Plex Sans", toc: true, gfm: true });
    print(`md → pdf (sans body font): wrote ${sans_out}`);
} catch(e) {
    print(`md → pdf (sans): skipped (${e})`);
}

// Typst TOC entries are plain text by default (inline code/bold/italic in
// a heading is flattened in the outline, while the body heading keeps its
// formatting). Set `toc_plain: false` to mirror heading formatting in the
// outline instead — matches the CLI's `--no-toc-plain`.
let formatted_toc_out = "/tmp/recon-doc-formatted-toc.pdf";
try {
    md_to_pdf(md, formatted_toc_out, #{ toc: true, gfm: true, toc_plain: false });
    print(`md → pdf (formatted TOC): wrote ${formatted_toc_out}`);
} catch(e) {
    print(`md → pdf (formatted TOC): skipped (${e})`);
}

// 3. One-shot md → pdf with a different destination. Insert `-2`
// before the `.pdf` extension so the file still opens with a normal
// PDF viewer (writing `.pdf.2` confuses macOS / Finder).
let pdf_out2 = if pdf_out.ends_with(".pdf") {
    pdf_out.sub_string(0, pdf_out.len - 4) + "-2.pdf"
} else {
    pdf_out + "-2.pdf"
};
try {
    md_to_pdf(md, pdf_out2, #{ toc: true, gfm: true });
    print(`md → pdf: wrote ${pdf_out2}`);
} catch(e) {
    print(`md → pdf: skipped (${e})`);
}

return 0;
