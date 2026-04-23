// Usage: recon --script hsts [CACHE_PATH]
//
// Demonstrates HSTS caching via the opts.hsts key. First request hits
// https:// and the server's STS header populates the cache. A follow-up
// http:// request to the same host is auto-upgraded. The cache file is
// curl-compatible plain TSV; scripts can share it with external tools.

let cache = if args.len() > 1 { args[1] } else { "/tmp/recon-hsts-demo.txt" };

// Request 1: https:// to cloudflare. Server returns STS header which
// populates the cache.
let r1 = http("https://www.cloudflare.com/", #{ hsts: cache });
print(`1st (https://): status=${r1.status}  hsts-populated`);

// Request 2: http:// to the same host — cache hit upgrades to https://.
let r2 = http("http://www.cloudflare.com/", #{ hsts: cache });
print(`2nd (http://):  final_url=${r2.final_url}  (should be https://)`);

// The cache file is human-readable — print its contents via file_read.
let cache_bytes = file_read(cache);
let cache_text = cache_bytes.as_string();
print(`--- ${cache} ---`);
print(cache_text);

return 0;
