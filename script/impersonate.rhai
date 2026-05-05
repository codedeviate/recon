// impersonate.rhai — browser TLS+H2 fingerprint impersonation.
//
// Usage: recon --script impersonate [PROFILE] [URL]
//
// Requires a build with `--features impersonate` (BoringSSL via rquest).
// The default build will reject the `impersonate` opts key with a clear
// rebuild hint.
//
// Profile names use rquest_util's underscore+dot convention:
//   chrome_131, firefox_128, safari_17.5, edge_131, okhttp_5,
//   chrome_android_131, safari_ios_17.4.1, ...
// Hyphens are also accepted (chrome-131 == chrome_131).

let profile = if args.len() > 1 { args[1] } else { "chrome_131" };
let url     = if args.len() > 2 { args[2] } else { "https://tls.peet.ws/api/all" };

let r = http(url, #{ impersonate: profile });
print(`profile: ${profile}`);
print(`status:  ${r.status}`);
print(`url:     ${r.url}`);
print(`body:    ${r.body.len()} bytes`);
