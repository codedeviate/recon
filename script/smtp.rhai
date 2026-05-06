// Usage: recon --script smtp [URL]
//
// Probe an SMTP server. Defaults to a reachable public relay; pass a
// URL like `smtp://localhost:25/` to target a local MailHog / test
// server. Reports EHLO capabilities + STARTTLS availability + AUTH
// mechanisms. Does NOT send a message — set opts.mail_from/mail_to to
// exercise delivery.

let url = if args.len() > 1 { args[1] } else { "smtp://gmail-smtp-in.l.google.com:25/" };

// Reachability probe on the default port before the SMTP dialog.
// Extract host[:port] using sub_string + index_of, since String::replace
// and split() in Rhai are mutating and can't be chained.
let host_port = url;
let i = host_port.index_of("://");
if i >= 0 { host_port = host_port.sub_string(i + 3); }
let at = host_port.index_of("@");
if at >= 0 { host_port = host_port.sub_string(at + 1); }
let slash = host_port.index_of("/");
if slash >= 0 { host_port = host_port.sub_string(0, slash); }
if host_port.index_of(":") < 0 { host_port += ":25"; }
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = smtp(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }  connect=${r.connect_ms.to_int()}ms`);
print(`capabilities: ${r.capabilities.len()} advertised`);
for c in r.capabilities { print(`  ${c}`); }

if r.starttls_ok != () {
    print(`STARTTLS: ${r.starttls_ok}`);
}
if r.auth_methods.len() > 0 {
    print(`AUTH: ${r.auth_methods}`);
}

return 0;
