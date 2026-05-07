// Usage: recon --script redis [URL [COMMAND]]
//
// Redis PING (default) or arbitrary RESP command.
// Guards on local-port reachability before invoking.

let url = if args.len() > 1 { args[1] } else { "redis://127.0.0.1:6379/" };
// Light pre-check: if 127.0.0.1:6379 isn't listening, exit 2 cleanly.
if url == "redis://127.0.0.1:6379/" {
    // `tcp()` raises on connect failure (refused / timeout), so a plain
    // `if !t.ok` guard never fires — wrap in try/catch.
    let reachable = false;
    try {
        let t = tcp("tcp://127.0.0.1:6379", #{ timeout: 2 });
        reachable = t.ok;
    } catch(e) { reachable = false; }
    if !reachable {
        print("no redis on 127.0.0.1:6379 — skipping");
        return 2;
    }
}
let r = if args.len() > 2 {
    redis(url, args[2])
} else {
    redis(url)
};
print(`${r.command}: ${r.reply} (${r.command_ms} ms)`);
return 0;
