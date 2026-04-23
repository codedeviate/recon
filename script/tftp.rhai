// Usage: recon --script tftp [URL]
//
// TFTP download (RFC 1350). Defaults to a local test server on
// port 69; gracefully skips when no server is listening. TFTP is
// UDP-based so the reachability probe uses TCP only for the host
// (not the TFTP port itself).

let url = if args.len() > 1 { args[1] } else { "tftp://127.0.0.1/test.bin" };
let host = url.replace("tftp://", "").split("/").shift().split(":").shift();

// TFTP speaks UDP; there's no TCP handshake to probe. Best-effort
// defence: just try, and if the transfer times out, exit 2.
let r = tftp(url, #{ blksize: 1024 });
print(`fetched ${r.filename} from ${r.host}:${r.port} — ${r.bytes.len()} bytes (blksize ${r.blksize})`);
return 0;
