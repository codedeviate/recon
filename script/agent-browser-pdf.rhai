// Usage: recon --script agent-browser-pdf [URL] [OUTPUT_PATH]
//
// Render a fully-rendered web page to PDF via Chrome's printToPDF.
// Differs from `recon --md-to-pdf` / `recon --html-to-pdf` in that the
// page is loaded by a real browser session, so client-side JS, lazy-loaded
// assets, web fonts, and dynamic charts all run before the PDF is taken.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
let out = if args.len() > 2 { args[2] } else { "/tmp/recon-page.pdf" };

agentBrowser::open(url);

// Simple form: render to file.
agentBrowser::pdf(out);
print(`saved: ${out}`);

// With per-call options (0.75.0). For example, force a wider viewport
// before rendering (via browser_args), or set a custom user-agent so
// the PDF reflects what that UA would see.
let out2 = `${out}.wide.pdf`;
agentBrowser::pdf(out2, #{
    user_agent: "Recon-pdf/1.0",
    browser_args: ["--window-size=1920,1080"],
});
print(`saved wide: ${out2}`);

agentBrowser::close();
return 0;
