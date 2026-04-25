// range.rhai — byte-range requests.
//
// Usage: recon --script range
//
// The `range` opts key sets the Range header using curl's syntax:
//   "0-1023"   first 1 KiB
//   "2048-"    from offset to EOF
//   "-512"     last 512 bytes
//
// Pair with `max_filesize` to gate pathologically large bodies.

// Fetch first 256 bytes.
let r = http("https://httpbin.org/bytes/4096", #{
    range: "0-255",
    max_filesize: "10M",
});
print(`first 256 bytes: ${r.body.len()} bytes, status ${r.status}`);

// Last 16 bytes (servers that support Range return 206 Partial Content).
let r = http("https://httpbin.org/bytes/4096", #{
    range: "-15",
});
print(`last 16 bytes: ${r.body.len()} bytes, status ${r.status}`);

return 0;
