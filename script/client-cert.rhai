// client-cert.rhai — present a client cert during the TLS handshake.
//
// Usage: recon --script client-cert [CERT_PATH] [KEY_PATH]
//
// Combined PEM (one file with both chain + key): omit the key arg.
// Split: pass cert and key separately.

let url = if args.len() > 3 { args[3] } else { "https://client.badssl.com/" };
let cert = if args.len() > 1 { args[1] } else { "./badssl.com.pem" };

if !file_exists(cert) {
    print(`cert ${cert} not found — skipping. Provide a real PEM via:`);
    print(`  recon --script client-cert /path/to/cert.pem [/path/to/key.pem]`);
    return 2;
}

let opts = #{ client_cert: cert };
if args.len() > 2 {
    opts.client_key = args[2];
}

let r = http(url, opts);
print(`${r.status} ${r.url}`);
print(`body: ${r.body.len()} bytes`);
