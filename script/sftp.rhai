// Usage: recon --script sftp [URL]
//
// SFTP probe. Needs either --ssh-key (or scripts-side opts.ssh_key)
// for key-based auth, or URL userinfo with a password — use with care.
// Default URL targets rebex.net's demo SFTP server (demo / password).

let url = if args.len() > 1 { args[1] } else { "sftp://demo:password@test.rebex.net/" };
let host_port = url.replace("sftp://", "").split("@").pop().split("/").shift();
if !host_port.contains(":") { host_port += ":22"; }
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
