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

// Simple form: render the current page to file.
agentBrowser::pdf(out);
print(`saved: ${out}`);

// With per-call options (0.75.0). Note: `agent-browser pdf` itself
// only renders the *current* page — `--user-agent` and `--args` are
// launch-time options, so they belong on `open()`. Putting them on
// `pdf()` would make agent-browser launch a fresh (blank) session
// with the requested args and then render that, producing an empty
// PDF. The right pattern is open-with-opts, then plain pdf:
let out2 = `${out}.wide.pdf`;
agentBrowser::open(url, #{
    user_agent: "Recon-pdf/1.0",
    browser_args: ["--window-size=1920,1080"],
});
agentBrowser::pdf(out2);
print(`saved wide: ${out2}`);

try { agentBrowser::close(); } catch(e) { /* already closed */ }
return 0;
