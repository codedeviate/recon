// Usage: recon --script agent-browser [URL]
//
// Minimal end-to-end browser flow: open, snapshot interactive elements,
// close. Guards on agent-browser availability so the script exits
// cleanly when the binary isn't installed.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}
let url = if args.len() > 1 { args[1] } else { "https://example.com" };
agentBrowser::open(url);
let title = agentBrowser::get("title");
let snap = agentBrowser::snapshot(true);
agentBrowser::close();
print(`${url}: ${title.title}`);
print(`  interactive elements captured (${json_stringify(snap).len()} bytes)`);
return 0;
