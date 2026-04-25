// retry.rhai — retry patterns against a transient endpoint.
//
// Usage: recon --script retry [URL]
//
// Shows the --retry cluster via the http() opts map:
//   retry             : number of retries
//   retry_all_errors  : widen predicate to include 4xx / parse errors
//   retry_connrefused : retry ECONNREFUSED
//   retry_delay       : fixed delay (seconds); otherwise exponential backoff
//   retry_max_time    : total wall-clock budget

let url = if args.len() > 1 { args[1] } else { "https://httpbin.org/status/503,503,200" };

print(`fetching ${url} with retry up to 3 attempts…`);
let r = http(url, #{
    retry: 3,
    retry_delay: 1,        // fixed 1-second delay (vs exponential 1s/2s/4s/…)
    retry_max_time: 30,
});
print(`final status: ${r.status}`);

// Retry all errors — including 4xx, which the default policy skips.
let r = http("https://httpbin.org/status/429", #{
    retry: 2,
    retry_all_errors: true,
    retry_delay: 1,
});
print(`429 retry: final status ${r.status}`);

// Liveness-wait pattern: retry on connrefused for 30 seconds after
// starting a slow service.
// let r = http("http://localhost:8080/healthz", #{
//     retry: 30,
//     retry_connrefused: true,
//     retry_delay: 1,
// });

return 0;
