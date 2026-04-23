// Usage: recon --script pop3 [URL]
//
// POP3 probe. Defaults to pop.gmail.com:995 (pop3s) for a pure
// capability survey — no auth, no retrieval. Add user:pass@ to the
// URL and a trailing /N to RETR message N.

let url = if args.len() > 1 { args[1] } else { "pop3s://pop.gmail.com/" };
let host_port = url.replace("pop3://", "").replace("pop3s://", "").split("@").pop().split("/").shift();
if !host_port.contains(":") {
    host_port += if url.starts_with("pop3s://") { ":995" } else { ":110" };
}
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = pop3(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }  rtt=${r.connect_ms.to_int()}ms`);
if r.banner != "" { print(r.banner.trim()); }
if r.capabilities.len() > 0 {
    print(`capabilities (${r.capabilities.len()}):`);
    for c in r.capabilities { print(`  ${c}`); }
}
if r.message_count != () {
    print(`mailbox: ${r.message_count} messages (${r.total_bytes} bytes)`);
}
return 0;
