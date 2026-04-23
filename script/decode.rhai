// decode.rhai — scan an image for a barcode / QR / DataMatrix / Aztec /
// PDF417 / MaxiCode and print the decoded payload.
//
// Usage:
//   recon --script decode [IMAGE]
//
// With no arg, the script generates a QR into /tmp and round-trips it
// through encode::decode so the example works without a supplied image.

if args.len() > 1 {
    let img = file_read(args[1]);
    let r = encode::decode(img);
    print(`format: ${r.format}`);
    print(`text:   ${r.text}`);
    return 0;
}

// Self-contained demo: encode → decode.
let png = encode::qr("hello from the script engine");
let r = encode::decode(png);
print(`format: ${r.format}`);
print(`text:   ${r.text}`);
return 0;
