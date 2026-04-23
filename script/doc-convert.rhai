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

// Load markdown via the existing file_read() binding.
let md = file_read(src).to_string();

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

// 3. One-shot md → pdf with a different destination.
try {
    md_to_pdf(md, pdf_out + ".2", #{ toc: true, gfm: true });
    print(`md → pdf: wrote ${pdf_out}.2`);
} catch(e) {
    print(`md → pdf: skipped (${e})`);
}

return 0;
