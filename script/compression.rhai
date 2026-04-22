// Usage: recon --script compression [PAYLOAD]
//
// Round-trip every stream-compression algo over a fixed payload.

let payload = if args.len() > 1 { args[1] } else { "the quick brown fox jumps over the lazy dog" };
let bytes = payload.to_blob();
print(`payload: ${bytes.len()} bytes`);
for algo in ["gzip", "deflate", "zstd", "brotli", "bzip2", "lz4", "xz", "snappy", "zlib"] {
    let c = compression::compress(algo, bytes);
    let back = compression::decompress(algo, c);
    assert(back == bytes, `${algo} round-trip failed`);
    print(`  ${algo}: ${bytes.len()} -> ${c.len()}B`);
}
return 0;
