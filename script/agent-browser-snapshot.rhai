// Usage: recon --script browser-snapshot URL
//
// Opens URL, takes an accessibility snapshot of interactive elements
// (what a vision-free AI agent would see), and pretty-prints the result.

if !agentBrowser::available {
    print("agent-browser not installed");
    return 2;
}

if args.len() < 2 {
    print(`usage: recon --script ${args[0]} URL`);
    return 1;
}

agentBrowser::open(args[1]);
let snap = agentBrowser::snapshot(true);   // true = interactive-only
agentBrowser::close();
print(json_stringify(snap, true));
return 0;
