// time-cond.rhai — conditional GETs via If-Modified-Since + ETag.
//
// Usage: recon --script time-cond [URL] [CACHE_PATH]
//
// Demonstrates three patterns that save bandwidth on reruns:
//
//   time_cond      : "If-Modified-Since" from date string OR file mtime
//                    (prefix with "-" for If-Unmodified-Since)
//   etag_compare   : read ETag from file, send as If-None-Match
//   etag_save      : save the response's ETag for a future round-trip
//   timestamping   : shortcut for time_cond = <-o target's mtime>

let url = if args.len() > 1 { args[1] } else { "https://httpbin.org/etag/recon-demo" };
let etag_path = if args.len() > 2 { args[2] } else { "/tmp/recon-etag.txt" };

// First fetch: save the ETag for later.
let r = http(url, #{ etag_save: etag_path });
print(`first fetch: ${r.status}`);

// Second fetch: compare against the saved ETag. Server returns 304
// if nothing changed, with an empty body — zero-bandwidth refresh.
// Only attempt the comparison if the first fetch actually wrote a
// cache file (some servers may not have sent an ETag).
if file_exists(etag_path) {
    let r = http(url, #{
        etag_compare: etag_path,
        etag_save: etag_path,  // also refresh the cache for next time
    });
    print(`conditional: ${r.status} (304 means unchanged)`);
} else {
    print(`conditional: skipped — ${etag_path} not created (server sent no ETag)`);
}

// Date-based conditional.
let r = http(url, #{ time_cond: "Wed, 21 Oct 2026 07:28:00 GMT" });
print(`date conditional: ${r.status}`);

return 0;
