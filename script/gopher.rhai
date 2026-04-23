// Usage: recon --script gopher [URL]
//
// Gopher selector fetch (RFC 1436). Defaults to gopher.floodgap.com's
// root — the canonical Gopher test server. Content is printed as
// a String plus available as r.bytes for binary handling.

let url = if args.len() > 1 { args[1] } else { "gopher://gopher.floodgap.com/" };
let host_port = url.replace("gopher://", "").replace("gophers://", "").split("/").shift();
if !host_port.contains(":") { host_port += ":70"; }
let t = tcp(`tcp://${host_port}`);
if !t.ok {
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
