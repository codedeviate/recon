// Usage: recon --script ping [HOST [COUNT]]
//
// TCP ping to host:port, or ICMP ping to a bare host.

let host = if args.len() > 1 { args[1] } else { "127.0.0.1" };
let count = if args.len() > 2 { parse_int(args[2]) } else { 3 };
let r = ping(host, count);
print(`${r.protocol} ${r.host}: ${r.received}/${r.sent} (${r.loss_pct}% loss)`);
if r.received > 0 {
    print(`  rtt min/avg/max = ${r.min_ms}/${r.avg_ms}/${r.max_ms} ms`);
}
return if r.received > 0 { 0 } else { 1 };
