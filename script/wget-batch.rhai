// wget-batch.rhai — exercise the 0.67.0 wget-style opts.
//
// Usage: recon --script wget-batch [URL]
//
// Demonstrates the four script-engine opts added in 0.67.0:
//   - wait    (seconds; CLI-only effect, accepted in opts for symmetry)
//   - tries   (total attempts; overrides the `retry` opt)
//   - accept  (filename-suffix accept list)
//   - reject  (filename-suffix reject list)
//
// Single-URL `http()` calls don't multi-iterate, so `wait` is a no-op
// per-call and `accept`/`reject` only filter when the URL would be
// iterated as part of an --input-file batch on the CLI. Keys are still
// accepted so scripts and CLI invocations have a shared opts surface.

let url = if args.len() > 1 { args[1] } else { "https://httpbin.org/get" };

// Wget-style retries: 5 total attempts allowed, with a 1s fixed delay.
// `tries` overrides any `retry` value also passed in.
let r = http(url, #{
    tries: 5,
    retry_delay: 1,
    wait: 0,
    accept: "json,html,htm",
    reject: "thumb,bak,tmp",
});

print(`status: ${r.status}`);
if r.status >= 200 && r.status < 400 {
    print("OK");
    return 0;
}
print(`unexpected status ${r.status}`);
return 1;
