// Usage: recon --script pop3 [URL]
//
// POP3 probe. Defaults to pop.gmail.com:995 (pop3s) for a pure
// capability survey — no auth, no retrieval. Add user:pass@ to the
// URL and a trailing /N to RETR message N.

let url = if args.len() > 1 { args[1] } else { "pop3s://pop.gmail.com/" };

// Extract host[:port] from the URL using sub_string + index_of, since
// String::replace and split() in Rhai are mutating and can't be chained.
let host_port = url;
let i = host_port.index_of("://");
if i >= 0 { host_port = host_port.sub_string(i + 3); }
let at = host_port.index_of("@");
if at >= 0 { host_port = host_port.sub_string(at + 1); }
let slash = host_port.index_of("/");
if slash >= 0 { host_port = host_port.sub_string(0, slash); }
if host_port.index_of(":") < 0 {
    host_port += if url.starts_with("pop3s://") { ":995" } else { ":110" };
}
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = pop3(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }  rtt=${r.connect_ms.to_int()}ms`);
// Rhai's String::trim() is mutating (returns ()); copy then trim in place.
if r.banner != "" {
    let banner = r.banner;
    banner.trim();
    print(banner);
}
if r.capabilities.len() > 0 {
    print(`capabilities (${r.capabilities.len()}):`);
    for c in r.capabilities { print(`  ${c}`); }
}
if r.message_count != () {
    print(`mailbox: ${r.message_count} messages (${r.total_bytes} bytes)`);
}
return 0;
