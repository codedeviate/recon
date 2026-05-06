// Usage: recon --script browser-screenshot URL [OUT_PATH]
//
// Opens URL in a browser via agent-browser, writes a screenshot to
// OUT_PATH (or agent-browser's default location), then closes.

if !agentBrowser::available {
    print("agent-browser not installed. Install via `brew install agent-browser`.");
    return 2;
}

if args.len() < 2 {
    print(`usage: recon --script ${args[0]} URL [OUT_PATH]`);
    return 1;
}

let url = args[1];
agentBrowser::open(url);

let out = if args.len() >= 3 {
    agentBrowser::screenshot(args[2])
} else {
    agentBrowser::screenshot()
};

try { agentBrowser::close(); } catch(e) { /* already closed */ }

// Rhai's String::trim() is mutating (returns ()); call as a separate
// statement, then print.
let cleaned = out;
cleaned.trim();
print(cleaned);
return 0;
