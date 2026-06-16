// Usage: recon --script http [URL]
//
// Minimal HTTP GET example. Fetches URL (default https://example.com),
// asserts 200, prints the body length.

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
let r = http(url);
assert(r.status == 200, `expected 200 from ${url}, got ${r.status}`);
print(`${r.final_url}: ${r.status} (${r.body.len()} bytes, ${r.duration_ms}ms)`);

// TLS hardening opts mirror the CLI flags. `curves` restricts the
// key-exchange groups; `pinnedpubkey` pins the server key (sha256//<base64>,
// ';'-separated for multiple). Both build a custom rustls config and reject
// the connection on mismatch.
//   let pinned = http(url, #{ pinnedpubkey: "sha256//BASE64HASH", curves: "X25519:P-256" });

return 0;
