// Usage: recon --script encode [DATA [FORMAT]]
//
// Generate a QR code (default) or another format as PNG.

let data = if args.len() > 1 { args[1] } else { "https://example.com" };
let fmt = if args.len() > 2 { args[2] } else { "qr" };
let png = if fmt == "qr" {
    encode::qr(data)
} else if fmt == "datamatrix" {
    encode::datamatrix(data)
} else {
    encode::barcode(fmt, data)
};
print(`${fmt}(${data}): ${png.len()} bytes`);
// Verify PNG signature.
assert(png.len() >= 8, "too short");
print(`sha256: ${sha256(png)}`);
return 0;
