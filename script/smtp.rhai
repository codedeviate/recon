// Usage: recon --script smtp [URL]
//
// Probe an SMTP server. Defaults to a reachable public relay; pass a
// URL like `smtp://localhost:25/` to target a local MailHog / test
// server. Reports EHLO capabilities + STARTTLS availability + AUTH
// mechanisms. Does NOT send a message — set opts.mail_from/mail_to to
// exercise delivery.

let url = if args.len() > 1 { args[1] } else { "smtp://gmail-smtp-in.l.google.com:25/" };

// Reachability probe on the default port before the SMTP dialog.
let host_port = url.replace("smtp://", "").replace("smtps://", "").split("/").shift();
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
