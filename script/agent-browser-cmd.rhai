// Usage: recon --script agent-browser-cmd [URL]
//
// `agentBrowser::cmd(args_array)` is the escape hatch for any agent-browser
// subcommand that doesn't have a typed Rhai wrapper yet. Pass the argv as
// a Rhai array of strings; stdout returns as a String. Lets scripts reach
// the full agent-browser CLI surface (network routing, storage, tabs,
// cookies, dashboard, etc.) without waiting for a binding update.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
agentBrowser::open(url);

// Cookies — get, set, clear.
let cookies = agentBrowser::cmd(["cookies", "get"]);
print(`cookies: ${cookies}`);

// agent-browser's `cookies set` takes name and value as two separate
// positional args (`set <name> <value>`), not a single "name=value"
// pair. Passing them joined produces a CDP "Invalid cookie fields"
// error from Network.setCookies.
agentBrowser::cmd([
    "cookies", "set", "session", "abc123",
    "--url", url,
    "--httpOnly",
]);

// Storage (localStorage / sessionStorage).
agentBrowser::cmd(["storage", "local"]);

// Tabs.
agentBrowser::cmd(["tab", "list"]);
agentBrowser::cmd(["tab", "new"]);

// Network — capture a snapshot of recent requests.
let requests = agentBrowser::cmd(["network", "requests"]);
print(`network requests: ${requests.len()} bytes captured`);

// Console / errors — useful for AI-driven debugging.
let console = agentBrowser::cmd(["console"]);
let errors = agentBrowser::cmd(["errors"]);
print(`console output bytes: ${console.len()}`);
print(`page errors bytes: ${errors.len()}`);

// Mouse primitives without selector wrappers.
agentBrowser::cmd(["mouse", "move", "100", "200"]);
agentBrowser::cmd(["mouse", "wheel", "120"]);

// Diff — compare current snapshot with the previous one.
try {
    let diff = agentBrowser::cmd(["diff", "snapshot"]);
    print(`snapshot diff: ${diff}`);
} catch(e) { print(`diff: ${e}`); }

// Close.
agentBrowser::cmd(["close"]);
return 0;
