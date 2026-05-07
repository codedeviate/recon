// Usage: recon --script gopher [URL]
//
// Gopher selector fetch (RFC 1436). Defaults to gopher.floodgap.com's
// root — the canonical Gopher test server. Content is printed as
// a String plus available as r.bytes for binary handling.

let url = if args.len() > 1 { args[1] } else { "gopher://gopher.floodgap.com/" };

// Extract host[:port] from the URL using sub_string + index_of, since
// String::replace and split() in Rhai are mutating and can't be chained.
let host_port = url;
let i = host_port.index_of("://");
if i >= 0 { host_port = host_port.sub_string(i + 3); }
let at = host_port.index_of("@");
if at >= 0 { host_port = host_port.sub_string(at + 1); }
let slash = host_port.index_of("/");
if slash >= 0 { host_port = host_port.sub_string(0, slash); }
if host_port.index_of(":") < 0 { host_port += ":70"; }
// `tcp()` raises on connect failure / timeout, so a plain `if !t.ok`
// guard never fires — wrap in try/catch and treat any failure as
// "host unreachable".
let reachable = false;
try {
    let t = tcp(`tcp://${host_port}`, #{ timeout: 5 });
    reachable = t.ok;
} catch(e) { reachable = false; }
if !reachable {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = gopher(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }  selector=${r.selector}`);
print(`content: ${r.content.len()} bytes`);
// Print the first 10 lines to show the map.
let lines = r.content.split("\n");
let max = if lines.len() < 10 { lines.len() } else { 10 };
for i in 0..max {
    print(`  ${lines[i]}`);
}
return 0;
