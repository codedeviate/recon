// render.rhai — turn HTML into readable text (text-browser view).
//
// html_to_text(html) renders with safe defaults (no ANSI, terminal/80
// column width). Pass an opts map to control width and colour.

let html = "<h1>Report</h1><p>See <a href=\"https://example.com\">example</a>.</p>";

let text = html_to_text(html);
print(text);

let narrow = html_to_text(html, #{ width: 40, color: "never" });
print(narrow);

// Fetch + render in one HTTP call: the body comes back already rendered.
let resp = http("https://example.com", #{ render: true, width: 100 });
print(resp.body);
