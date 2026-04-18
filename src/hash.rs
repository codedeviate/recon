//! `--hash <ALGO>`: stream a source through a chosen hasher and print the
//! digest in hex, base64, or raw bytes. Backed by the source layer, so the
//! input can be a file, `file://` URL, HTTP URL, or stdin.

use anyhow::{anyhow, Result};

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
}
