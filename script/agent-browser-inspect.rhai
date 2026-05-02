// Usage: recon --script agent-browser-inspect [URL]
//
// Read-only inspection surface: snapshot variants, eval(js), get(what), get
// (what, sel), and predicates (is_visible / is_enabled / is_checked).
// Useful as the first step in any agent-browser flow — figure out what's
// on the page before acting.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
agentBrowser::open(url);

// ── snapshot ─────────────────────────────────────────────────────────────────
// snapshot()             — full accessibility tree (interactive + structural)
// snapshot(true)         — interactive elements only (smaller, AI-friendly)
// snapshot(opts)         — apply 0.75.0 global options for this call only
let full = agentBrowser::snapshot();
let interactive = agentBrowser::snapshot(true);
print(`full snapshot: ${json_stringify(full).len()} bytes`);
print(`interactive only: ${json_stringify(interactive).len()} bytes`);

// ── eval_js — run arbitrary JS in the page context ──────────────────────────
// Note: Rhai reserves `eval` as a keyword, so the binding is exposed
// under the alias `eval_js`. Both names exist (the typed `eval` is for
// Rust callers); scripts must use `eval_js`.
let title = agentBrowser::eval_js("document.title");
print(`title via eval_js: ${title}`);

let counts = agentBrowser::eval_js(`({
    links: document.querySelectorAll('a').length,
    images: document.querySelectorAll('img').length,
    inputs: document.querySelectorAll('input').length
})`);
print(`counts: ${counts}`);

// eval_js with per-call options (0.75.0): override the user-agent for this
// one call only, without disturbing module defaults.
let ua = agentBrowser::eval_js("navigator.userAgent", #{ user_agent: "Recon-eval/1.0" });
print(`UA (per-call override): ${ua}`);

// ── get(what) and get(what, sel) ─────────────────────────────────────────────
// `what` ∈ {text, html, value, attr <name>, title, url, count, box, styles, cdp-url}
let url_info = agentBrowser::get("url");
print(`current url: ${url_info.url}`);

let title_info = agentBrowser::get("title");
print(`current title: ${title_info.title}`);

try {
    let h1_text = agentBrowser::get("text", "h1");
    print(`h1 text: ${h1_text.text}`);
} catch(e) { print(`get text: ${e}`); }

try {
    let h1_html = agentBrowser::get("html", "h1");
    print(`h1 html: ${h1_html.html}`);
} catch(e) { print(`get html: ${e}`); }

try {
    let count = agentBrowser::get("count", "a");
    print(`<a> count: ${count.count}`);
} catch(e) { print(`get count: ${e}`); }

// ── Predicates ───────────────────────────────────────────────────────────────
try {
    let v = agentBrowser::is_visible("h1");
    print(`h1 visible: ${v}`);
} catch(e) { print(`is_visible: ${e}`); }

try {
    let e = agentBrowser::is_enabled("a");
    print(`first link enabled: ${e}`);
} catch(e) { print(`is_enabled: ${e}`); }

try {
    let c = agentBrowser::is_checked("input[type='checkbox']");
    print(`checkbox checked: ${c}`);
} catch(e) { print(`is_checked: ${e}`); }

agentBrowser::close();
return 0;
