// pdf.rhai — pdf_export_page demo.
//
// Usage: recon --script pdf [PDF_PATH] [PAGE]
//
// Renders the given page of the PDF to PNG, JPEG and WEBP in /tmp.
// Falls back to docs/MANUAL.pdf shipped with the project so the script
// is runnable with no arguments.

let src = if args.len() > 1 { args[1] } else { "docs/MANUAL.pdf" };
let page = if args.len() > 2 { parse_int(args[2]) } else { 1 };

// 1. Default PNG to file.
let png_path = `/tmp/recon-pdf-page-${page}.png`;
try {
    pdf_export_page(src, page, png_path);
    print(`png: wrote ${png_path}`);
} catch(e) {
    print(`png: skipped (${e})`);
    return 0;  // agent-browser missing — skip the rest
}

// 2. JPEG at custom quality + larger viewport.
let jpg_path = `/tmp/recon-pdf-page-${page}.jpg`;
pdf_export_page(src, page, jpg_path, #{
    viewport: "1920x2715",
    scale: 2,
    quality: 70,
});
print(`jpeg: wrote ${jpg_path}`);

// 3. WEBP from a Blob (no destination — bytes returned).
let webp_bytes = pdf_export_page(src, page, #{ format: "webp", quality: 80 });
print(`webp: got ${webp_bytes.len()} bytes`);

return 0;
