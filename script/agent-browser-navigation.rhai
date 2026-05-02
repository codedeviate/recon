// Usage: recon --script agent-browser-navigation [URL_A] [URL_B]
//
// Multi-page flow demonstrating browser history navigation:
// open, back, forward, reload, plus close / close_all.

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let a = if args.len() > 1 { args[1] } else { "https://example.com" };
let b = if args.len() > 2 { args[2] } else { "https://example.org" };

// First page.
agentBrowser::open(a);
print(`opened: ${agentBrowser::get("url").url}`);

// Second page (creates a history entry).
agentBrowser::open(b);
print(`navigated: ${agentBrowser::get("url").url}`);

// Step backward through history.
agentBrowser::back();
print(`back to: ${agentBrowser::get("url").url}`);

// Step forward again.
agentBrowser::forward();
print(`forward to: ${agentBrowser::get("url").url}`);

// Force a reload (bypasses the in-memory page state).
agentBrowser::reload();
print(`reloaded: ${agentBrowser::get("url").url}`);

// Close just this session.
agentBrowser::close();

// Or, when you've spawned multiple sessions and want a clean slate:
// agentBrowser::close_all();

return 0;
