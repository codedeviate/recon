//! `--compress <ALGO>` / `--decompress [ALGO]`: streaming compression and
//! decompression over any source (file, URL, stdin, file://). Five
//! algorithms: gzip, deflate, zstd, brotli, bzip2.

use anyhow::{anyhow, Result};

/// Supported compression algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Gzip,
    Deflate,
    Zstd,
    Brotli,
    Bzip2,
}

impl Algo {
    pub fn canonical(&self) -> &'static str {
        match self {
            Algo::Gzip => "gzip",
            Algo::Deflate => "deflate",
            Algo::Zstd => "zstd",
            Algo::Brotli => "brotli",
            Algo::Bzip2 => "bzip2",
        }
    }

    /// Lowercase accepted aliases (not including the canonical name).
    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            Algo::Gzip => &["gz"],
            Algo::Deflate => &[],
            Algo::Zstd => &["zst"],
            Algo::Brotli => &["br"],
            Algo::Bzip2 => &["bz2"],
        }
    }

    /// Native level range (inclusive), per the library's own quality scale.
    pub fn level_range(&self) -> (u32, u32) {
        match self {
            Algo::Gzip => (0, 9),
            Algo::Deflate => (0, 9),
            Algo::Zstd => (1, 22),
            Algo::Brotli => (0, 11),
            Algo::Bzip2 => (1, 9),
        }
    }

    /// Library default level.
    pub fn default_level(&self) -> u32 {
        match self {
            Algo::Gzip => 6,
            Algo::Deflate => 6,
            Algo::Zstd => 3,
            Algo::Brotli => 4,
            Algo::Bzip2 => 6,
        }
    }

    /// Magic-byte prefix for auto-detect. `None` = no magic bytes (deflate, brotli).
    pub fn magic(&self) -> Option<&'static [u8]> {
        match self {
            Algo::Gzip => Some(&[0x1f, 0x8b]),
            Algo::Zstd => Some(&[0x28, 0xb5, 0x2f, 0xfd]),
            Algo::Bzip2 => Some(&[0x42, 0x5a, 0x68]),
            Algo::Deflate | Algo::Brotli => None,
        }
    }

    pub const ALL: &'static [Algo] = &[
        Algo::Gzip,
        Algo::Deflate,
        Algo::Zstd,
        Algo::Brotli,
        Algo::Bzip2,
    ];
}

/// Parse a user-supplied algorithm name. Case-insensitive; accepts both
/// canonical names and the per-algo alias list. Unknown input lists all
/// canonical names.
pub fn parse_algo(input: &str) -> Result<Algo> {
    let lower = input.trim().to_ascii_lowercase();
    for algo in Algo::ALL {
        if algo.canonical() == lower || algo.aliases().iter().any(|a| *a == lower) {
            return Ok(*algo);
        }
    }
    let supported: Vec<&str> = Algo::ALL.iter().map(|a| a.canonical()).collect();
    Err(anyhow!(
        "unknown compression algorithm '{input}'; supported: {}",
        supported.join(", "),
    ))
}

/// Inspect up to 6 bytes from the start of a stream and match against the
/// magic table. Returns the detected algorithm, or `None` if nothing matched
/// (including when the buffer is shorter than any magic prefix).
pub fn detect_from_magic(head: &[u8]) -> Option<Algo> {
    for algo in Algo::ALL {
        if let Some(magic) = algo.magic() {
            if head.len() >= magic.len() && &head[..magic.len()] == magic {
                return Some(*algo);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_algo_canonical_names() {
        assert_eq!(parse_algo("gzip").unwrap(), Algo::Gzip);
        assert_eq!(parse_algo("deflate").unwrap(), Algo::Deflate);
        assert_eq!(parse_algo("zstd").unwrap(), Algo::Zstd);
        assert_eq!(parse_algo("brotli").unwrap(), Algo::Brotli);
        assert_eq!(parse_algo("bzip2").unwrap(), Algo::Bzip2);
    }

    #[test]
    fn parse_algo_aliases() {
        assert_eq!(parse_algo("gz").unwrap(), Algo::Gzip);
        assert_eq!(parse_algo("zst").unwrap(), Algo::Zstd);
        assert_eq!(parse_algo("br").unwrap(), Algo::Brotli);
        assert_eq!(parse_algo("bz2").unwrap(), Algo::Bzip2);
    }

    #[test]
    fn parse_algo_case_insensitive() {
        assert_eq!(parse_algo("GZIP").unwrap(), Algo::Gzip);
        assert_eq!(parse_algo("Zstd").unwrap(), Algo::Zstd);
        assert_eq!(parse_algo("BR").unwrap(), Algo::Brotli);
    }

    #[test]
    fn parse_algo_trims_whitespace() {
        assert_eq!(parse_algo("  gzip  ").unwrap(), Algo::Gzip);
    }

    #[test]
    fn parse_algo_unknown_lists_supported() {
        let err = parse_algo("snappy").unwrap_err().to_string();
        assert!(err.contains("snappy"), "got: {err}");
        assert!(err.contains("gzip"), "got: {err}");
        assert!(err.contains("bzip2"), "got: {err}");
    }

    #[test]
    fn detect_from_magic_matches_gzip() {
        assert_eq!(detect_from_magic(&[0x1f, 0x8b, 0x08, 0x00]), Some(Algo::Gzip));
    }

    #[test]
    fn detect_from_magic_matches_zstd() {
        assert_eq!(
            detect_from_magic(&[0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x00]),
            Some(Algo::Zstd),
        );
    }

    #[test]
    fn detect_from_magic_matches_bzip2() {
        assert_eq!(detect_from_magic(b"BZh91AY&"), Some(Algo::Bzip2));
    }

    #[test]
    fn detect_from_magic_no_match() {
        assert_eq!(detect_from_magic(b"hello"), None);
        assert_eq!(detect_from_magic(&[]), None);
        assert_eq!(detect_from_magic(&[0x1f]), None); // too short for gzip
    }

    #[test]
    fn algo_all_has_five_variants() {
        assert_eq!(Algo::ALL.len(), 5);
    }

    #[test]
    fn algo_level_ranges() {
        assert_eq!(Algo::Gzip.level_range(), (0, 9));
        assert_eq!(Algo::Zstd.level_range(), (1, 22));
        assert_eq!(Algo::Brotli.level_range(), (0, 11));
        assert_eq!(Algo::Bzip2.level_range(), (1, 9));
    }

    #[test]
    fn algo_default_levels() {
        assert_eq!(Algo::Gzip.default_level(), 6);
        assert_eq!(Algo::Zstd.default_level(), 3);
        assert_eq!(Algo::Brotli.default_level(), 4);
        assert_eq!(Algo::Bzip2.default_level(), 6);
    }
}
