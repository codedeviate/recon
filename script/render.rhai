// render.rhai — turn HTML into readable text (text-browser view).
//
// html_to_text(html) renders with safe defaults (no ANSI, terminal/80
// column width). Pass an opts map to control width and colour.

let html = "<h1>Report</h1><p>See <a href=\"https://example.com\">example</a>.</p>";

let text = html_to_text(html);
print(text);

let narrow = html_to_text(html, #{ width: 40, color: "never" });
print(narrow);

// no_links: true drops the [N] footnote markers + URL reference list
// (plain) or the inline link styling (coloured) — anchor text stays.
let clean = html_to_text(html, #{ no_links: true });
print(clean);

// Fetch + render in one HTTP call: the body comes back already rendered.
// render_no_links carries the same toggle through the http() opts map.
let resp = http("https://example.com", #{ render: true, width: 100, render_no_links: true });
print(resp.body);
