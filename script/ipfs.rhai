// Usage: recon --script ipfs [CID_OR_URL]
//
// Demonstrates IPFS URL rewriting. Scripts can:
//   1. Build the gateway URL with ipfs_url() and use http() normally.
//   2. Hand http() an ipfs:// URL — recon rewrites before dispatch.
// Default gateway is https://ipfs.io; override via opts.gateway.

let cid = if args.len() > 1 { args[1] } else { "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi" };
let url = if cid.starts_with("ipfs://") || cid.starts_with("ipns://") { cid } else { `ipfs://${cid}` };

let gw_url = ipfs_url(url);
print(`gateway URL: ${gw_url}`);

// Fetch via http(). On a CLI invocation, `recon ${url}` would go through
// the same rewrite+HTTP path — no separate ipfs() function needed.
let r = http(gw_url);
print(`status:   ${r.status}`);
print(`size:     ${r.body_bytes.len()} bytes`);
print(`charset:  ${r.charset ?? "()"}`);

return 0;
