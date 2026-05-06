// Usage: recon --script agent-browser-find [URL]
//
// Demonstrates `agentBrowser::find(locator, value, action)` — agent-browser's
// semantic locator API, which finds elements by role / text / label / etc.
// instead of by CSS selector. Each call returns a JSON envelope; on failure
// (no matching element) the call raises a Rhai error that scripts can
// catch with `try { ... } catch { ... }`.
//
// Locators (per `agent-browser find --help`):
//   role         — ARIA role: button, link, textbox, checkbox, ...
//   text         — visible text content
//   label        — associated <label> text
//   placeholder  — input placeholder attribute
//   alt          — image alt text
//   title        — title attribute
//   testid       — data-testid attribute
//   first, last  — positional shortcuts (no value)
//   nth          — value is the index
//
// Actions: click, dblclick, hover, focus, fill, type, check, uncheck.
// Use the 4-arg form `find(loc, val, action, text)` for fill / type.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
agentBrowser::open(url);

// Each find pattern. Wrapped in try/catch so missing elements don't
// abort the demo — real scripts would let the error propagate.

// 1. By role + accessible name (the most reliable locator for buttons / links)
try {
    agentBrowser::find("role", "link", "click");
    print("clicked first link by role");
} catch(e) {
    print(`role/link/click: ${e}`);
}

// 2. By visible text. Useful when the role/name pair is awkward.
try {
    agentBrowser::find("text", "More information...", "click");
    print("clicked 'More information...' by text");
} catch(e) {
    print(`text/click: ${e}`);
}

// Steps 1–2 may have clicked a real link and navigated (or closed
// the target outright on some pages). Re-open the original URL so
// the input-focused locators below run against a known-good page.
// Bare `back()` would throw "Inspected target navigated or closed"
// when the target is in a bad state.
try {
    agentBrowser::open(url);
} catch(e) {
    print(`re-open after click: ${e}`);
}

// 3. By label — finds an input via its <label> text.
try {
    agentBrowser::find("label", "Email", "fill", "user@example.com");
    print("filled email field by label");
} catch(e) {
    print(`label/fill: ${e}`);
}

// 4. By placeholder — useful for inputs without a visible label.
try {
    agentBrowser::find("placeholder", "Search…", "fill", "recon");
    print("filled search by placeholder");
} catch(e) {
    print(`placeholder/fill: ${e}`);
}

// 5. By alt — typically images.
try {
    agentBrowser::find("alt", "Logo", "click");
    print("clicked image with alt='Logo'");
} catch(e) {
    print(`alt/click: ${e}`);
}

// 6. By title attribute.
try {
    agentBrowser::find("title", "Edit", "click");
    print("clicked element with title='Edit'");
} catch(e) {
    print(`title/click: ${e}`);
}

// 7. By data-testid — the convention used by many React/Vue test suites.
try {
    agentBrowser::find("testid", "submit-button", "click");
    print("clicked submit by testid");
} catch(e) {
    print(`testid/click: ${e}`);
}

// 8. Positional: first / last / nth.
try {
    agentBrowser::find("first", "input", "focus");
    print("focused first input");
} catch(e) {
    print(`first/focus: ${e}`);
}
try {
    agentBrowser::find("nth", "2", "click");   // index 2 (0-based or 1-based per agent-browser)
    print("clicked nth=2");
} catch(e) {
    print(`nth/click: ${e}`);
}

// `close()` is best-effort — the target may already be in a closed
// or navigation-churn state from earlier clicks.
try { agentBrowser::close(); } catch(e) { /* already closed */ }
return 0;
