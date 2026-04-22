// Usage: recon --script http [URL]
//
// Minimal HTTP GET example. Fetches URL (default https://example.com),
// asserts 200, prints the body length.

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
let r = http(url);
assert(r.status == 200, `expected 200 from ${url}, got ${r.status}`);
print(`${r.final_url}: ${r.status} (${r.body.len()} bytes, ${r.duration_ms}ms)`);
return 0;
