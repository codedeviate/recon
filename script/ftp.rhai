// Usage: recon --script ftp [URL]
//
// Anonymous FTP probe. Default: rebex.net (classic public FTP/FTPS
// demo server). Lists a directory when the URL ends with / or retrieves
// a file otherwise. Gracefully exits 2 when the host is unreachable.

let url = if args.len() > 1 { args[1] } else { "ftp://test.rebex.net/pub/example/" };
let host_port = url.replace("ftp://", "").replace("ftps://", "").split("/").shift();
if !host_port.contains(":") { host_port += ":21"; }
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

// rebex.net requires demo/password credentials; anonymous works on most
// public mirrors. Override via the opts map as needed.
let r = ftp(url, #{ user: "demo", pass: "password" });
print(`${r.host}:${r.port}${ if r.tls { " (TLS)" } else { "" } }  user=${r.user}  pwd=${r.pwd}`);
if r.mode == "list" {
    print(`listing (${r.listing.len()} entries):`);
    for entry in r.listing { print(`  ${entry}`); }
} else {
    print(`retrieved ${r.bytes.len()} bytes`);
}
return 0;
