// Usage: recon --script browser-guard URL
//
// Demonstrates the "prefer browser, fall back to http" pattern.
// If agent-browser is installed, use it for a full-page screenshot + title.
// Otherwise, probe the URL with recon's http() and print basic status.

if args.len() < 2 {
    print(`usage: recon --script ${args[0]} URL`);
    return 1;
}

let url = args[1];

if agentBrowser::available {
    agentBrowser::open(url);
    let result = agentBrowser::get("title");
    agentBrowser::close();
    print(`[browser] title = ${result.title}`);
    return 0;
}

// Fallback: plain HTTP probe. Pure bytes, no JS execution, but lets the
// script do something useful even without agent-browser installed.
let r = http(url);
print(`[http] status=${r.status} duration_ms=${r.duration_ms}`);
return if r.status == 200 { 0 } else { 1 };
