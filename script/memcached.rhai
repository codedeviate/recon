// Usage: recon --script memcached [URL]
//
// Memcached text-protocol probe: version + optional /stats.
// Guards on local-port reachability.

let url = if args.len() > 1 { args[1] } else { "memcached://127.0.0.1:11211/stats" };
if url.contains("127.0.0.1:11211") {
    // `tcp()` raises on connect failure (refused / timeout), so a plain
    // `if !t.ok` guard never fires — wrap in try/catch.
    let reachable = false;
    try {
        let t = tcp("tcp://127.0.0.1:11211", #{ timeout: 2 });
        reachable = t.ok;
    } catch(e) { reachable = false; }
    if !reachable {
        print("no memcached on 127.0.0.1:11211 — skipping");
        return 2;
    }
}
let r = memcached(url);
print(`${r.host}:${r.port}  connect=${r.connect_ms}ms  ${r.version}`);
if r.stats.len() > 0 {
    print(`  ${r.stats.len()} stats fields`);
}
return 0;
