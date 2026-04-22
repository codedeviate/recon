// Usage: recon --script browser-multi [HOST]
//
// Multiple browsers in one script, each with independent state.
// Useful for scraping the same site under different identities or
// for A/B-style checks. Rhai is single-threaded, so calls are
// sequential — "parallel" here means independent cookie jars and
// headers, not concurrent I/O.

let host = if args.len() > 1 { args[1] } else { "https://httpbin.org" };

// Three personas, three cookie jars.
let bot = browser(#{
    user_agent: "recon-bot/1.0",
    headers: #{ "X-Persona": "bot" },
});
let user = browser(#{
    user_agent: "Mozilla/5.0 (recon-user-demo)",
    headers: #{ "X-Persona": "user" },
});
let mobile = browser(#{
    user_agent: "Mozilla/5.0 (iPhone; recon-mobile-demo)",
    headers: #{ "X-Persona": "mobile" },
});

// Each browser collects its own cookie.
bot.get(`${host}/cookies/set/id/bot-session`);
user.get(`${host}/cookies/set/id/user-session`);
mobile.get(`${host}/cookies/set/id/mobile-session`);

// Fetch the same endpoint from each — the server sees three
// distinct identities and three distinct cookie values.
for b in [bot, user, mobile] {
    let r = b.get(`${host}/cookies`);
    let jar_size = b.cookies().len();
    print(`${b.cookies()[0].value}  (jar: ${jar_size} cookie, status ${r.status})`);
}

// Isolation check: bot's jar does not leak into user's jar.
let bot_cookies = bot.cookies();
let user_cookies = user.cookies();
assert(bot_cookies[0].value != user_cookies[0].value, "jars leaked!");
print("✓ browser jars are isolated");

return 0;
