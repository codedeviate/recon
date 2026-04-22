// Usage: recon --script browser [HOST]
//
// Demonstrates the browser() handle: configure once, reuse across
// multiple requests, cookies + headers stick automatically.

let host = if args.len() > 1 { args[1] } else { "https://httpbin.org" };

let b = browser();
b.set_user_agent("recon-browser-example/1.0");
b.set_header("X-Demo", "yes");

// First request: ask the server to set a cookie.
let r1 = b.get(`${host}/cookies/set/session/abc123`);
print(`set cookie: ${r1.status}`);

// Second request: the browser replays the cookie automatically.
let r2 = b.get(`${host}/cookies`);
print(`replay:     ${r2.status}`);
if r2.body.contains("abc123") {
    print("  \u2713 cookie stuck across calls");
}

// Inspect the jar directly.
let jar = b.cookies();
print(`jar has ${jar.len()} cookie(s)`);

// Run a second browser in parallel with its own isolated state.
let b2 = browser();
let r3 = b2.get(`${host}/cookies`);
print(`second browser (no cookies): ${r3.status}`);

return 0;
