// Usage: recon --script imap [URL]
//
// IMAP capability probe. Defaults to imap.gmail.com:993 (imaps) — no
// auth, reports advertised capabilities only. Add user:pass@ and a
// mailbox path (e.g. imap://user:pass@host/INBOX) to select + report
// EXISTS/RECENT; append `;UID=N` to fetch a specific message.

let url = if args.len() > 1 { args[1] } else { "imaps://imap.gmail.com/" };
let host_port = url.replace("imap://", "").replace("imaps://", "").split("@").pop().split("/").shift();
if !host_port.contains(":") {
    host_port += if url.starts_with("imaps://") { ":993" } else { ":143" };
}
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = imap(url);
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }`);
print(`capabilities (${r.capabilities.len()}): ${r.capabilities.join(", ")}`);
if r.mailbox != () {
    print(`mailbox: ${r.mailbox}  exists=${r.exists}  recent=${r.recent}`);
}
if r.mailboxes != () {
    print(`mailboxes (${r.mailboxes.len()}):`);
    for m in r.mailboxes { print(`  ${m}`); }
}
return 0;
