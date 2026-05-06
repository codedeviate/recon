// Usage: recon --script sftp [URL]
//
// SFTP probe. Needs either --ssh-key (or scripts-side opts.ssh_key)
// for key-based auth, or URL userinfo with a password — use with care.
// Default URL targets rebex.net's demo SFTP server (demo / password).

let url = if args.len() > 1 { args[1] } else { "sftp://demo:password@test.rebex.net/" };

// Extract host[:port] from the URL using sub_string + index_of, since
// String::replace and split() in Rhai are mutating and can't be chained.
let host_port = url;
let i = host_port.index_of("://");
if i >= 0 { host_port = host_port.sub_string(i + 3); }
let at = host_port.index_of("@");
if at >= 0 { host_port = host_port.sub_string(at + 1); }
let slash = host_port.index_of("/");
if slash >= 0 { host_port = host_port.sub_string(0, slash); }
if host_port.index_of(":") < 0 { host_port += ":22"; }
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`${host_port} unreachable — skipping`);
    return 2;
}

let r = sftp(url);
print(`${r.user}@${r.host}:${r.port}  connect=${r.connect_ms.to_int()}ms  path=${r.path}`);
if r.mode == "list" {
    print(`listing (${r.listing.len()} entries):`);
    for entry in r.listing {
        let kind = if entry.is_dir { "d" } else { "-" };
        print(`  ${kind} ${entry.size} ${entry.name}`);
    }
} else {
    print(`retrieved ${r.bytes.len()} bytes`);
}
return 0;
