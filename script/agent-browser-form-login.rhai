// Usage: recon --script browser-form-login URL USER_SEL USER PASS_SEL PASS SUBMIT_SEL
//
// Pattern demo. Fills a username field, a password field, and clicks
// a submit button. Selectors may be CSS (`#email`), XPath, or
// agent-browser refs (`@e3`) from a prior snapshot.

if !agentBrowser::available {
    print("agent-browser not installed");
    return 2;
}

if args.len() < 7 {
    print(`usage: recon --script ${args[0]} URL USER_SEL USER PASS_SEL PASS SUBMIT_SEL`);
    return 1;
}

agentBrowser::open(args[1]);
agentBrowser::fill(args[2], args[3]);
agentBrowser::fill(args[4], args[5]);
agentBrowser::click(args[6]);
agentBrowser::wait("2000");                 // let any redirect settle
print(`final url: ${agentBrowser::get("url").url}`);
agentBrowser::close();
return 0;
