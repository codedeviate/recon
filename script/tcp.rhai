// Usage: recon --script tcp [HOST:PORT]
//
// TCP connect probe with latency reporting.

let target = if args.len() > 1 { args[1] } else { "example.com:443" };
let r = tcp(`tcp://${target}`);
print(`${r.host}:${r.port} -> ${r.resolved_ip} (${r.duration_ms}ms)`);
return if r.ok { 0 } else { 1 };
