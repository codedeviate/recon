// Usage: recon --script memcached [URL]
//
// Memcached text-protocol probe: version + optional /stats.
// Guards on local-port reachability.

let url = if args.len() > 1 { args[1] } else { "memcached://127.0.0.1:11211/stats" };
if url.contains("127.0.0.1:11211") {
    let t = tcp("tcp://127.0.0.1:11211");
    if !t.ok {
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
