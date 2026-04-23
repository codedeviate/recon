// Usage: recon --script proxy [PROXY_URL] [TARGET_URL]
//
// Demonstrates the http() opts-map proxy plumbing. Every proxy flag on
// the CLI has a snake_case equivalent on the opts map: `proxy`,
// `proxy_user`, `noproxy`, `proxy_insecure`, `proxy_cacert`. The same
// URL schemes work: http://, https://, socks5://, socks5h://.

let proxy_url = if args.len() > 1 { args[1] } else { "" };
let target = if args.len() > 2 { args[2] } else { "https://httpbin.org/ip" };

if proxy_url == "" {
    // No proxy: plain direct fetch showing baseline.
    let r = http(target);
    print(`direct: ${r.status}  ${r.body_bytes.len()} bytes`);
    return 0;
}

// With proxy. http() will transparently tunnel or TLS-to-proxy based
// on the proxy URL's scheme.
let r = http(target, #{
    proxy: proxy_url,
    // Uncomment for basic auth:
    // proxy_user: "alice:secret",
    // Skip cert verification on a self-signed https:// proxy:
    // proxy_insecure: true,
});
print(`via ${proxy_url}: ${r.status}  ${r.body_bytes.len()} bytes`);
return if r.status >= 200 && r.status < 400 { 0 } else { 1 };
