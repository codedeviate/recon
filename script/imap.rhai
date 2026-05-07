// Usage: recon --script imap [URL]
//
// IMAP capability probe. Defaults to imap.gmail.com:993 (imaps) — no
// auth, reports advertised capabilities only. Add user:pass@ and a
// mailbox path (e.g. imap://user:pass@host/INBOX) to select + report
// EXISTS/RECENT; append `;UID=N` to fetch a specific message.

let url = if args.len() > 1 { args[1] } else { "imaps://imap.gmail.com/" };

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
    host_port += if url.starts_with("imaps://") { ":993" } else { ":143" };
}
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

let r = imap(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }`);
// Rhai's Array doesn't expose .join() in this build — concatenate manually.
let caps = "";
for c in r.capabilities {
    if caps.len() > 0 { caps += ", "; }
    caps += c;
}
print(`capabilities (${r.capabilities.len()}): ${caps}`);
if r.mailbox != () {
    print(`mailbox: ${r.mailbox}  exists=${r.exists}  recent=${r.recent}`);
}
if r.mailboxes != () {
    print(`mailboxes (${r.mailboxes.len()}):`);
    for m in r.mailboxes { print(`  ${m}`); }
}
return 0;
