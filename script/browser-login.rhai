// Usage: recon --script browser-login [HOST]
//
// Sends a JSON login request, captures the session cookie, then
// fetches a protected resource on the same browser — the cookie
// is replayed automatically. Map body auto-serialises to JSON.

let host = if args.len() > 1 { args[1] } else { "https://httpbin.org" };

let b = browser();
b.set_user_agent("recon-login-demo/1.0");

// Step 1: log in. The Map body becomes application/json automatically.
// httpbin.org doesn't actually validate credentials — it just echoes
// the body — but it does set a Set-Cookie header on /cookies/set.
let login = b.post(`${host}/post`, #{
    username: "demo",
    password: "hunter2",
});
print(`login: ${login.status}`);

// Step 2: simulate a session cookie being set (httpbin's /cookies/set
// endpoint Set-Cookies whatever we ask).
b.get(`${host}/cookies/set/session_token/abc123`);

// Step 3: fetch a protected resource. The browser replays the cookie
// captured in step 2 automatically — no manual header management.
let profile = b.get(`${host}/cookies`);
print(`profile: ${profile.status}`);
if profile.body.contains("abc123") {
    print("  ✓ session cookie replayed");
} else {
    print("  ✗ cookie not replayed");
    return 1;
}

return 0;
