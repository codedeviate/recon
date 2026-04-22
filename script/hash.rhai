// Usage: recon --script hash [PAYLOAD]
//
// Show every hash algorithm over a fixed payload.

let payload = if args.len() > 1 { args[1] } else { "the quick brown fox" };
print(`payload: ${payload.len()} bytes`);
for algo in ["md5", "sha1", "sha256", "sha384", "sha512", "sha3_256", "sha3_512", "blake3", "crc32"] {
    let digest = hash(algo, payload);
    print(`  ${algo}: ${digest}`);
}
return 0;
