// Usage: recon --script browser-title URL
//
// Opens URL and prints the page title via agent-browser's `get title` command.

if !agentBrowser::available {
    print("agent-browser not installed");
    return 2;
}

if args.len() < 2 {
    print(`usage: recon --script ${args[0]} URL`);
    return 1;
}

agentBrowser::open(args[1]);
let result = agentBrowser::get("title");
agentBrowser::close();
// `get` returns the agent-browser payload map; the `title` field holds
// the actual page title.
print(result.title);
return 0;
