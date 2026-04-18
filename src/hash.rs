//! `--hash <ALGO>`: stream a source through a chosen hasher and print the
//! digest in hex, base64, or raw bytes. Backed by the source layer, so the
//! input can be a file, `file://` URL, HTTP URL, or stdin.

use anyhow::{anyhow, Result};
use std::io::Write;

/// Supported hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Sha3_256,
    Sha3_512,
    Blake3,
}

impl Algo {
    /// Canonical name for display and listing.
    pub fn canonical(&self) -> &'static str {
        match self {
            Algo::Md5 => "md5",
            Algo::Sha1 => "sha1",
            Algo::Sha256 => "sha256",
            Algo::Sha384 => "sha384",
            Algo::Sha512 => "sha512",
            Algo::Sha3_256 => "sha3-256",
            Algo::Sha3_512 => "sha3-512",
            Algo::Blake3 => "blake3",
        }
    }

    /// Digest size in bits.
    pub fn bits(&self) -> u32 {
        match self {
            Algo::Md5 => 128,
            Algo::Sha1 => 160,
            Algo::Sha256 => 256,
            Algo::Sha384 => 384,
            Algo::Sha512 => 512,
            Algo::Sha3_256 => 256,
            Algo::Sha3_512 => 512,
            Algo::Blake3 => 256,
        }
    }

    /// All algorithms, in display order.
    pub const ALL: &'static [Algo] = &[
        Algo::Md5,
        Algo::Sha1,
        Algo::Sha256,
        Algo::Sha384,
        Algo::Sha512,
        Algo::Sha3_256,
        Algo::Sha3_512,
        Algo::Blake3,
    ];
}

/// Output format for the digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Hex,
    Base64,
    Raw,
}

/// Parse a user-supplied algorithm name. Case-insensitive; accepts the
/// canonical form plus common hyphen/underscore variants. Unknown input
/// errors with the supplied string plus the list of supported names.
pub fn parse_algo(input: &str) -> Result<Algo> {
    let lower = input.trim().to_ascii_lowercase();
    let out = match lower.as_str() {
        "md5" => Algo::Md5,
        "sha1" | "sha-1" | "sha_1" => Algo::Sha1,
        "sha256" | "sha-256" | "sha_256" => Algo::Sha256,
        "sha384" | "sha-384" | "sha_384" => Algo::Sha384,
        "sha512" | "sha-512" | "sha_512" => Algo::Sha512,
        "sha3-256" | "sha3_256" | "sha3256" => Algo::Sha3_256,
        "sha3-512" | "sha3_512" | "sha3512" => Algo::Sha3_512,
        "blake3" => Algo::Blake3,
        _ => {
            let supported: Vec<&str> =
                Algo::ALL.iter().map(|a| a.canonical()).collect();
            return Err(anyhow!(
                "unknown hash algorithm '{input}'; supported: {}",
                supported.join(", "),
            ));
        }
    };
    Ok(out)
}

/// Parse a user-supplied format name. Case-insensitive.
pub fn parse_format(input: &str) -> Result<Format> {
    match input.trim().to_ascii_lowercase().as_str() {
        "hex" => Ok(Format::Hex),
        "base64" => Ok(Format::Base64),
        "raw" => Ok(Format::Raw),
        _ => Err(anyhow!(
            "unknown hash format '{input}'; expected hex, base64, or raw"
        )),
    }
}

use digest::Digest;

/// Internal streaming hasher state. Each variant wraps the per-algorithm
/// hasher type so we can `update` and `finalize` without chasing trait
/// objects with associated types.
pub enum HasherKind {
    Md5(md5::Md5),
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
    Sha3_256(sha3::Sha3_256),
    Sha3_512(sha3::Sha3_512),
    Blake3(blake3::Hasher),
}

impl HasherKind {
    pub fn for_algo(algo: Algo) -> Self {
        match algo {
            Algo::Md5 => HasherKind::Md5(md5::Md5::new()),
            Algo::Sha1 => HasherKind::Sha1(sha1::Sha1::new()),
            Algo::Sha256 => HasherKind::Sha256(sha2::Sha256::new()),
            Algo::Sha384 => HasherKind::Sha384(sha2::Sha384::new()),
            Algo::Sha512 => HasherKind::Sha512(sha2::Sha512::new()),
            Algo::Sha3_256 => HasherKind::Sha3_256(sha3::Sha3_256::new()),
            Algo::Sha3_512 => HasherKind::Sha3_512(sha3::Sha3_512::new()),
            Algo::Blake3 => HasherKind::Blake3(blake3::Hasher::new()),
        }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            HasherKind::Md5(h) => h.update(bytes),
            HasherKind::Sha1(h) => h.update(bytes),
            HasherKind::Sha256(h) => h.update(bytes),
            HasherKind::Sha384(h) => h.update(bytes),
            HasherKind::Sha512(h) => h.update(bytes),
            HasherKind::Sha3_256(h) => h.update(bytes),
            HasherKind::Sha3_512(h) => h.update(bytes),
            HasherKind::Blake3(h) => {
                h.update(bytes);
            }
        }
    }

    pub fn finalize(self) -> Vec<u8> {
        match self {
            HasherKind::Md5(h) => h.finalize().to_vec(),
            HasherKind::Sha1(h) => h.finalize().to_vec(),
            HasherKind::Sha256(h) => h.finalize().to_vec(),
            HasherKind::Sha384(h) => h.finalize().to_vec(),
            HasherKind::Sha512(h) => h.finalize().to_vec(),
            HasherKind::Sha3_256(h) => h.finalize().to_vec(),
            HasherKind::Sha3_512(h) => h.finalize().to_vec(),
            HasherKind::Blake3(h) => h.finalize().as_bytes().to_vec(),
        }
    }
}

/// Stream `reader` through a fresh hasher for `algo`, returning the digest.
/// Uses an 8 KiB chunked read so memory stays constant.
pub fn compute(algo: Algo, reader: &mut dyn std::io::Read) -> Result<Vec<u8>> {
    let mut hasher = HasherKind::for_algo(algo);
    let mut buf = [0u8; 8 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// Write a digest to `out` in the chosen format.
/// - Hex → lowercase hex, trailing `\n`.
/// - Base64 → standard base64 with padding, trailing `\n`.
/// - Raw → digest bytes verbatim, NO trailing newline.
pub fn write_digest(
    out: &mut dyn Write,
    digest: &[u8],
    format: Format,
) -> std::io::Result<()> {
    match format {
        Format::Hex => {
            let mut s = String::with_capacity(digest.len() * 2);
            for b in digest {
                s.push_str(&format!("{:02x}", b));
            }
            writeln!(out, "{s}")?;
        }
        Format::Base64 => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
            writeln!(out, "{encoded}")?;
        }
        Format::Raw => {
            out.write_all(digest)?;
        }
    }
    Ok(())
}

/// Print the `--hash-list` output to `out`.
pub fn print_list(out: &mut dyn Write) -> std::io::Result<()> {
    // Width: the canonical name column is 10 chars wide — enough for
    // "sha3-256" (8) + margin.
    for algo in Algo::ALL {
        writeln!(out, "{:<10} {}-bit", algo.canonical(), algo.bits())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_algo_all_canonical_names() {
        assert_eq!(parse_algo("md5").unwrap(), Algo::Md5);
        assert_eq!(parse_algo("sha1").unwrap(), Algo::Sha1);
        assert_eq!(parse_algo("sha256").unwrap(), Algo::Sha256);
        assert_eq!(parse_algo("sha384").unwrap(), Algo::Sha384);
        assert_eq!(parse_algo("sha512").unwrap(), Algo::Sha512);
        assert_eq!(parse_algo("sha3-256").unwrap(), Algo::Sha3_256);
        assert_eq!(parse_algo("sha3-512").unwrap(), Algo::Sha3_512);
        assert_eq!(parse_algo("blake3").unwrap(), Algo::Blake3);
    }

    #[test]
    fn parse_algo_case_insensitive() {
        assert_eq!(parse_algo("MD5").unwrap(), Algo::Md5);
        assert_eq!(parse_algo("SHA-256").unwrap(), Algo::Sha256);
        assert_eq!(parse_algo("Sha3-512").unwrap(), Algo::Sha3_512);
        assert_eq!(parse_algo("BLAKE3").unwrap(), Algo::Blake3);
    }

    #[test]
    fn parse_algo_underscore_variants() {
        assert_eq!(parse_algo("sha_1").unwrap(), Algo::Sha1);
        assert_eq!(parse_algo("sha_256").unwrap(), Algo::Sha256);
        assert_eq!(parse_algo("sha3_256").unwrap(), Algo::Sha3_256);
        assert_eq!(parse_algo("sha3256").unwrap(), Algo::Sha3_256);
    }

    #[test]
    fn parse_algo_trims_whitespace() {
        assert_eq!(parse_algo("  sha256  ").unwrap(), Algo::Sha256);
    }

    #[test]
    fn parse_algo_unknown_lists_supported() {
        let err = parse_algo("sha2").unwrap_err().to_string();
        assert!(err.contains("sha2"), "got: {err}");
        assert!(err.contains("md5"), "got: {err}");
        assert!(err.contains("blake3"), "got: {err}");
    }

    #[test]
    fn parse_format_happy_paths() {
        assert_eq!(parse_format("hex").unwrap(), Format::Hex);
        assert_eq!(parse_format("base64").unwrap(), Format::Base64);
        assert_eq!(parse_format("raw").unwrap(), Format::Raw);
    }

    #[test]
    fn parse_format_case_insensitive() {
        assert_eq!(parse_format("HEX").unwrap(), Format::Hex);
        assert_eq!(parse_format("Base64").unwrap(), Format::Base64);
    }

    #[test]
    fn parse_format_unknown_errors() {
        let err = parse_format("binary").unwrap_err().to_string();
        assert!(err.contains("binary"), "got: {err}");
        assert!(err.contains("hex"), "got: {err}");
    }

    #[test]
    fn algo_canonical_and_bits_table() {
        assert_eq!(Algo::Md5.canonical(), "md5");
        assert_eq!(Algo::Md5.bits(), 128);
        assert_eq!(Algo::Sha3_256.canonical(), "sha3-256");
        assert_eq!(Algo::Sha3_256.bits(), 256);
        assert_eq!(Algo::Blake3.canonical(), "blake3");
        assert_eq!(Algo::Blake3.bits(), 256);
    }

    #[test]
    fn algo_all_covers_every_variant() {
        assert_eq!(Algo::ALL.len(), 8);
    }

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    fn compute_str(algo: Algo, input: &[u8]) -> String {
        let digest = compute(algo, &mut std::io::Cursor::new(input)).unwrap();
        hex(&digest)
    }

    #[test]
    fn vector_md5_empty() {
        assert_eq!(compute_str(Algo::Md5, b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn vector_md5_abc() {
        assert_eq!(compute_str(Algo::Md5, b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn vector_sha1_empty() {
        assert_eq!(compute_str(Algo::Sha1, b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn vector_sha1_abc() {
        assert_eq!(compute_str(Algo::Sha1, b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn vector_sha256_empty() {
        assert_eq!(
            compute_str(Algo::Sha256, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
    }

    #[test]
    fn vector_sha256_abc() {
        assert_eq!(
            compute_str(Algo::Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        );
    }

    #[test]
    fn vector_sha384_empty() {
        assert_eq!(
            compute_str(Algo::Sha384, b""),
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b",
        );
    }

    #[test]
    fn vector_sha384_abc() {
        assert_eq!(
            compute_str(Algo::Sha384, b"abc"),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
        );
    }

    #[test]
    fn vector_sha512_empty() {
        assert_eq!(
            compute_str(Algo::Sha512, b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
        );
    }

    #[test]
    fn vector_sha512_abc() {
        assert_eq!(
            compute_str(Algo::Sha512, b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        );
    }

    #[test]
    fn vector_sha3_256_empty() {
        assert_eq!(
            compute_str(Algo::Sha3_256, b""),
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
        );
    }

    #[test]
    fn vector_sha3_256_abc() {
        assert_eq!(
            compute_str(Algo::Sha3_256, b"abc"),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
        );
    }

    #[test]
    fn vector_sha3_512_abc() {
        assert_eq!(
            compute_str(Algo::Sha3_512, b"abc"),
            "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
        );
    }

    #[test]
    fn vector_blake3_empty() {
        assert_eq!(
            compute_str(Algo::Blake3, b""),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        );
    }

    #[test]
    fn vector_blake3_abc() {
        assert_eq!(
            compute_str(Algo::Blake3, b"abc"),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
        );
    }

    #[test]
    fn compute_streams_large_input_consistently() {
        // Exercise the 8 KiB chunked loop at least 12 times.
        let input = vec![0u8; 100 * 1024];

        // Reference: feed the whole buffer at once.
        let whole = compute(Algo::Sha256, &mut std::io::Cursor::new(input.clone())).unwrap();

        // Streaming via a reader that hands out tiny chunks (forces the
        // outer loop to spin many times).
        struct ChunkedReader<'a> {
            data: &'a [u8],
            chunk: usize,
            pos: usize,
        }
        impl<'a> std::io::Read for ChunkedReader<'a> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let remaining = self.data.len().saturating_sub(self.pos);
                let to_copy = remaining.min(self.chunk).min(buf.len());
                buf[..to_copy].copy_from_slice(&self.data[self.pos..self.pos + to_copy]);
                self.pos += to_copy;
                Ok(to_copy)
            }
        }

        let mut chunked = ChunkedReader { data: &input, chunk: 97, pos: 0 };
        let streamed = compute(Algo::Sha256, &mut chunked).unwrap();
        assert_eq!(whole, streamed);
    }

    #[test]
    fn write_digest_hex_has_trailing_newline() {
        let mut out = Vec::new();
        write_digest(&mut out, &[0xde, 0xad, 0xbe, 0xef], Format::Hex).unwrap();
        assert_eq!(out, b"deadbeef\n");
    }

    #[test]
    fn write_digest_base64_has_trailing_newline() {
        let mut out = Vec::new();
        write_digest(&mut out, &[0xde, 0xad, 0xbe, 0xef], Format::Base64).unwrap();
        assert_eq!(out, b"3q2+7w==\n");
    }

    #[test]
    fn write_digest_raw_has_no_trailing_newline() {
        let mut out = Vec::new();
        write_digest(&mut out, &[0xde, 0xad, 0xbe, 0xef], Format::Raw).unwrap();
        assert_eq!(out, &[0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn print_list_contains_all_algorithms() {
        let mut out = Vec::new();
        print_list(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        for algo in Algo::ALL {
            assert!(text.contains(algo.canonical()), "missing: {} in {text}", algo.canonical());
        }
        // Exactly 8 lines, no blank trailer.
        assert_eq!(text.lines().count(), 8, "output was:\n{text}");
    }

    #[test]
    fn print_list_includes_bit_sizes() {
        let mut out = Vec::new();
        print_list(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("128-bit"));  // md5
        assert!(text.contains("256-bit"));  // sha256, sha3-256, blake3
        assert!(text.contains("512-bit"));  // sha512, sha3-512
    }
}
