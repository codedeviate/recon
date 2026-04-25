// oauth2.rhai — OAuth 2.0 Bearer token auth.
//
// Usage: recon --script oauth2
//
// The oauth2_bearer opts key is sugar for an Authorization header.
// Pair with a real token loader (env var, HSM, vault, a preflight
// OAuth token-exchange request) as needed.

let token = env("API_TOKEN", "demo-token-1234");

let r = http("https://httpbin.org/bearer", #{
    oauth2_bearer: token,
});
print(`status: ${r.status}`);

// Token refresh pattern — if the first call returns 401, fetch a
// fresh token and retry once.
fn call_api(url, tok) {
    http(url, #{
        oauth2_bearer: tok,
        // tight timeouts so a dead auth server doesn't hang CI
        timeout_ms: 5000,
        retry: 1,
    })
}

let r = call_api("https://httpbin.org/bearer", token);
if r.status == 401 {
    print("token expired — fetching a fresh one");
    // let fresh = fetch_token(...);
    // let r = call_api(url, fresh);
}

return 0;
