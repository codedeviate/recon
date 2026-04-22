//! `--compress <ALGO>` / `--decompress [ALGO]`: streaming compression and
//! decompression over any source (file, URL, stdin, file://). Nine
//! algorithms: gzip, deflate, zstd, brotli, bzip2, lz4, xz, snappy, zlib.

use anyhow::{anyhow, Result};

/// Supported compression algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Gzip,
    Deflate,
    Zstd,
    Brotli,
    Bzip2,
    Lz4,
    Xz,
    Snappy,
    Zlib,
}

impl Algo {
    pub fn canonical(&self) -> &'static str {
        match self {
            Algo::Gzip => "gzip",
            Algo::Deflate => "deflate",
            Algo::Zstd => "zstd",
            Algo::Brotli => "brotli",
            Algo::Bzip2 => "bzip2",
            Algo::Lz4 => "lz4",
            Algo::Xz => "xz",
            Algo::Snappy => "snappy",
            Algo::Zlib => "zlib",
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
            Algo::Lz4 => &["lz"],
            Algo::Xz => &["lzma"],
            Algo::Snappy => &["snap", "sz"],
            Algo::Zlib => &["zl"],
        }
    }

    /// Native level range (inclusive), per the library's own quality scale.
    /// `(0, 0)` means "algorithm has no level setting".
    pub fn level_range(&self) -> (u32, u32) {
        match self {
            Algo::Gzip => (0, 9),
            Algo::Deflate => (0, 9),
            Algo::Zstd => (1, 22),
            Algo::Brotli => (0, 11),
            Algo::Bzip2 => (1, 9),
            Algo::Xz => (0, 9),
            Algo::Zlib => (0, 9),
            Algo::Lz4 | Algo::Snappy => (0, 0),
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
            Algo::Xz => 6,
            Algo::Zlib => 6,
            Algo::Lz4 | Algo::Snappy => 0,
        }
    }

    /// True when the algorithm has no user-settable level. Scripts that
    /// pass `--compression-level` against these get a clear error.
    pub fn is_levelless(&self) -> bool {
        matches!(self, Algo::Lz4 | Algo::Snappy)
    }

    /// Magic-byte prefix for auto-detect. `None` = no magic bytes
    /// (deflate, brotli) or special-cased elsewhere (zlib — see
    /// `detect_from_magic`).
    pub fn magic(&self) -> Option<&'static [u8]> {
        match self {
            Algo::Gzip => Some(&[0x1f, 0x8b]),
            Algo::Zstd => Some(&[0x28, 0xb5, 0x2f, 0xfd]),
            Algo::Bzip2 => Some(&[0x42, 0x5a, 0x68]),
            Algo::Lz4 => Some(&[0x04, 0x22, 0x4d, 0x18]),
            Algo::Xz => Some(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]),
            Algo::Snappy => Some(&[0xff, 0x06, 0x00, 0x00, 0x73, 0x4e, 0x61, 0x50, 0x70, 0x59]),
            Algo::Deflate | Algo::Brotli | Algo::Zlib => None,
        }
    }

    pub const ALL: &'static [Algo] = &[
        Algo::Gzip,
        Algo::Deflate,
        Algo::Zstd,
        Algo::Brotli,
        Algo::Bzip2,
        Algo::Lz4,
        Algo::Xz,
        Algo::Snappy,
        Algo::Zlib,
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

/// Inspect the first bytes of a stream and match against the magic
/// table. Returns the detected algorithm, or `None` if nothing matched.
/// Zlib is a special case: its header is `CMF(0x78) + FLG` where
/// `(CMF*256 + FLG) % 31 == 0` — no constant-prefix match, so checked
/// after the table.
pub fn detect_from_magic(head: &[u8]) -> Option<Algo> {
    for algo in Algo::ALL {
        if let Some(magic) = algo.magic() {
            if head.len() >= magic.len() && &head[..magic.len()] == magic {
                return Some(*algo);
            }
        }
    }
    // Zlib header check. CMF byte is 0x78 in ~100% of real-world use
    // (32 KB window + deflate). The FLG byte's low 5 bits plus CMF's
    // 8 bits + FLG's upper 3 bits must make the 16-bit big-endian value
    // divisible by 31 (RFC 1950 §2.2).
    if head.len() >= 2 && head[0] == 0x78 {
        let fcheck = ((head[0] as u16) << 8) | head[1] as u16;
        if fcheck % 31 == 0 {
            return Some(Algo::Zlib);
        }
    }
    None
}

/// One of the five level-quality words. Case-insensitive when parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelWord {
    Fastest,
    Fast,
    Default,
    Good,
    Best,
}

impl LevelWord {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "fastest" => Some(LevelWord::Fastest),
            "fast" => Some(LevelWord::Fast),
            "default" => Some(LevelWord::Default),
            "good" => Some(LevelWord::Good),
            "best" => Some(LevelWord::Best),
            _ => None,
        }
    }
}

/// Resolved level value: either a word (resolved per-algo later) or a raw
/// number in the algorithm's native range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Word(LevelWord),
    Num(u32),
}

/// Parse a `--compression-level <LEVEL>` value into `Level`. A trimmed
/// decimal integer → `Level::Num`; a recognised word → `Level::Word`.
/// Anything else errors with both grammar forms.
pub fn parse_level(input: &str) -> Result<Level> {
    let trimmed = input.trim();
    if let Ok(n) = trimmed.parse::<u32>() {
        return Ok(Level::Num(n));
    }
    if let Some(word) = LevelWord::parse(trimmed) {
        return Ok(Level::Word(word));
    }
    Err(anyhow!(
        "unknown compression level '{input}'; \
         supported: number or fastest|fast|default|good|best"
    ))
}

/// Resolve a `Level` to the algorithm's native integer. Errors when a raw
/// number falls outside the algorithm's valid range.
pub fn resolve_native_level(algo: Algo, level: Level) -> Result<u32> {
    match level {
        Level::Num(n) => {
            let (min, max) = algo.level_range();
            if n < min || n > max {
                return Err(anyhow!(
                    "level {n} out of range for {} (valid: {}-{} or fastest|fast|default|good|best)",
                    algo.canonical(),
                    min,
                    max,
                ));
            }
            Ok(n)
        }
        Level::Word(word) => Ok(word_to_native(algo, word)),
    }
}

use std::io::Read;

const BROTLI_BUF_SIZE: usize = 8192;

/// Stream `source` through the chosen encoder, writing compressed bytes to
/// `out`. Most encoders are read-side wrappers; Lz4 is write-side
/// (lz4_flex's FrameEncoder wraps a Writer), handled as an early arm.
pub fn compress(
    algo: Algo,
    level: u32,
    mut source: Box<dyn Read>,
    out: &mut dyn std::io::Write,
) -> Result<u64> {
    if let Algo::Lz4 = algo {
        // FrameEncoder wraps the Writer; io::copy drains source into it.
        let mut encoder = lz4_flex::frame::FrameEncoder::new(out);
        let n = std::io::copy(&mut source, &mut encoder)?;
        encoder.finish().map_err(|e| anyhow!("lz4 finish: {e}"))?;
        return Ok(n);
    }
    let mut encoder: Box<dyn Read> = match algo {
        Algo::Gzip => Box::new(flate2::read::GzEncoder::new(
            source,
            flate2::Compression::new(level),
        )),
        Algo::Deflate => Box::new(flate2::read::DeflateEncoder::new(
            source,
            flate2::Compression::new(level),
        )),
        Algo::Zstd => Box::new(zstd::stream::read::Encoder::new(source, level as i32)?),
        Algo::Brotli => Box::new(brotli::CompressorReader::new(
            source,
            BROTLI_BUF_SIZE,
            level,
            // lgwin = 22 is brotli's default sliding-window size.
            22,
        )),
        Algo::Bzip2 => Box::new(bzip2::read::BzEncoder::new(
            source,
            bzip2::Compression::new(level),
        )),
        Algo::Xz => Box::new(xz2::read::XzEncoder::new(source, level)),
        Algo::Snappy => Box::new(snap::read::FrameEncoder::new(source)),
        Algo::Zlib => Box::new(flate2::read::ZlibEncoder::new(
            source,
            flate2::Compression::new(level),
        )),
        Algo::Lz4 => unreachable!("handled above"),
    };
    Ok(std::io::copy(&mut encoder, out)?)
}

/// Stream `source` through the chosen decoder, writing plain bytes to `out`.
/// Returns the number of bytes written.
pub fn decompress(
    algo: Algo,
    source: Box<dyn Read>,
    out: &mut dyn std::io::Write,
) -> Result<u64> {
    let mut decoder: Box<dyn Read> = match algo {
        Algo::Gzip => Box::new(flate2::read::GzDecoder::new(source)),
        Algo::Deflate => Box::new(flate2::read::DeflateDecoder::new(source)),
        Algo::Zstd => Box::new(zstd::stream::read::Decoder::new(source)?),
        Algo::Brotli => Box::new(brotli::Decompressor::new(source, BROTLI_BUF_SIZE)),
        Algo::Bzip2 => Box::new(bzip2::read::BzDecoder::new(source)),
        Algo::Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(source)),
        Algo::Xz => Box::new(xz2::read::XzDecoder::new(source)),
        Algo::Snappy => Box::new(snap::read::FrameDecoder::new(source)),
        Algo::Zlib => Box::new(flate2::read::ZlibDecoder::new(source)),
    };
    Ok(std::io::copy(&mut decoder, out)?)
}

fn word_to_native(algo: Algo, word: LevelWord) -> u32 {
    // Table from spec. Keep in sync with spec section "Level words".
    match (algo, word) {
        (Algo::Gzip, LevelWord::Fastest)     => 1,
        (Algo::Gzip, LevelWord::Fast)        => 3,
        (Algo::Gzip, LevelWord::Default)     => 6,
        (Algo::Gzip, LevelWord::Good)        => 7,
        (Algo::Gzip, LevelWord::Best)        => 9,
        (Algo::Deflate, LevelWord::Fastest)  => 1,
        (Algo::Deflate, LevelWord::Fast)     => 3,
        (Algo::Deflate, LevelWord::Default)  => 6,
        (Algo::Deflate, LevelWord::Good)     => 7,
        (Algo::Deflate, LevelWord::Best)     => 9,
        (Algo::Zstd, LevelWord::Fastest)     => 1,
        (Algo::Zstd, LevelWord::Fast)        => 3,
        (Algo::Zstd, LevelWord::Default)     => 3,
        (Algo::Zstd, LevelWord::Good)        => 9,
        (Algo::Zstd, LevelWord::Best)        => 22,
        (Algo::Brotli, LevelWord::Fastest)   => 0,
        (Algo::Brotli, LevelWord::Fast)      => 2,
        (Algo::Brotli, LevelWord::Default)   => 4,
        (Algo::Brotli, LevelWord::Good)      => 7,
        (Algo::Brotli, LevelWord::Best)      => 11,
        (Algo::Bzip2, LevelWord::Fastest)    => 1,
        (Algo::Bzip2, LevelWord::Fast)       => 3,
        (Algo::Bzip2, LevelWord::Default)    => 6,
        (Algo::Bzip2, LevelWord::Good)       => 7,
        (Algo::Bzip2, LevelWord::Best)       => 9,
        (Algo::Xz, LevelWord::Fastest)       => 1,
        (Algo::Xz, LevelWord::Fast)          => 3,
        (Algo::Xz, LevelWord::Default)       => 6,
        (Algo::Xz, LevelWord::Good)          => 7,
        (Algo::Xz, LevelWord::Best)          => 9,
        (Algo::Zlib, LevelWord::Fastest)     => 1,
        (Algo::Zlib, LevelWord::Fast)        => 3,
        (Algo::Zlib, LevelWord::Default)     => 6,
        (Algo::Zlib, LevelWord::Good)        => 7,
        (Algo::Zlib, LevelWord::Best)        => 9,
        // Level-less algorithms: the `resolve_native_level` path should
        // reject a user-supplied level before this is reached. If a word
        // sneaks through (e.g. from future call sites), return 0 — the
        // encoders for Lz4/Snappy ignore the level.
        (Algo::Lz4, _) | (Algo::Snappy, _) => 0,
    }
}

use std::io::Write;

/// Print the `--compress-list` output to `out`.
pub fn print_list(out: &mut dyn Write) -> std::io::Result<()> {
    for algo in Algo::ALL {
        let aliases = if algo.aliases().is_empty() {
            "—".to_string()
        } else {
            algo.aliases().join(",")
        };
        let magic = match algo.magic() {
            Some(bytes) => bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>(),
            None => "—".to_string(),
        };
        let (min, max) = algo.level_range();
        writeln!(
            out,
            "{:<8} aliases: {:<8} magic: {:<10} levels: {}-{:<4} default: {}",
            algo.canonical(),
            aliases,
            magic,
            min,
            max,
            algo.default_level(),
        )?;
    }
    Ok(())
}

use crate::cli::Args;

/// Top-level entry for `--compress` and `--decompress`. Exactly one of
/// those flags must be set; mutual exclusion is enforced in `main.rs`
/// before this is called.
pub fn run(args: &Args) -> Result<()> {
    if args.compress.is_some() {
        return run_compress(args);
    }
    if args.decompress.is_some() {
        return run_decompress(args);
    }
    Err(anyhow!("internal: compression::run called with neither flag set"))
}

fn run_compress(args: &Args) -> Result<()> {
    let algo_str = args.compress.as_deref().unwrap_or("");
    let algo = parse_algo(algo_str)?;

    let level = match args.compression_level.as_deref() {
        Some(s) => {
            if algo.is_levelless() {
                return Err(anyhow!(
                    "{}: algorithm has no level setting (remove --compression-level)",
                    algo.canonical()
                ));
            }
            let parsed = parse_level(s)?;
            resolve_native_level(algo, parsed)?
        }
        None => algo.default_level(),
    };

    let source_kind = crate::source::resolve(args)?;
    if args.verbose >= 1 {
        let label = source_label(&source_kind);
        eprintln!("* compress: {} level={} from {}", algo.canonical(), level, label);
    }
    let reader = crate::source::open(source_kind, args)?;

    let mut out: Box<dyn Write> = open_output(args)?;
    compress(algo, level, reader, &mut out)?;
    Ok(())
}

fn run_decompress(args: &Args) -> Result<()> {
    // `--compression-level` is only valid with --compress.
    if args.compression_level.is_some() {
        return Err(anyhow!("--compression-level only applies to --compress"));
    }

    let algo_flag = args.decompress.as_deref().unwrap_or("");
    let source_kind = crate::source::resolve(args)?;
    let mut reader = crate::source::open(source_kind.clone(), args)?;

    let algo = if algo_flag.is_empty() {
        // Peek the first 6 bytes, then chain them back with the rest.
        let mut head = [0u8; 6];
        let n = fill_buf(&mut reader, &mut head)?;
        let detected = detect_from_magic(&head[..n]).ok_or_else(|| anyhow!(
            "--decompress: could not auto-detect format from magic bytes; \
             supply an algorithm: --decompress <gzip|deflate|zstd|brotli|bzip2>"
        ))?;
        // Re-chain: prepend the peeked bytes.
        reader = Box::new(std::io::Read::chain(
            std::io::Cursor::new(head[..n].to_vec()),
            reader,
        ));
        detected
    } else {
        parse_algo(algo_flag)?
    };

    if args.verbose >= 1 {
        let label = source_label(&source_kind);
        eprintln!("* decompress: {} from {}", algo.canonical(), label);
    }

    let mut out: Box<dyn Write> = open_output(args)?;
    decompress(algo, reader, &mut out)?;
    Ok(())
}

fn source_label(kind: &crate::source::SourceKind) -> String {
    match kind {
        crate::source::SourceKind::Stdin => "stdin".to_string(),
        crate::source::SourceKind::File(p) => p.display().to_string(),
        crate::source::SourceKind::Http(u) => u.clone(),
    }
}

fn open_output(args: &Args) -> Result<Box<dyn Write>> {
    match &args.output {
        Some(path) => Ok(Box::new(std::fs::File::create(path)?)),
        None => Ok(Box::new(std::io::stdout().lock())),
    }
}

/// `Read::read` can return short reads; loop until we fill the buffer or
/// hit EOF. Returns the number of bytes actually read.
fn fill_buf(reader: &mut Box<dyn Read>, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = reader.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
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
        let err = parse_algo("supercompress").unwrap_err().to_string();
        assert!(err.contains("supercompress"), "got: {err}");
        assert!(err.contains("gzip"), "got: {err}");
        assert!(err.contains("bzip2"), "got: {err}");
        assert!(err.contains("snappy"), "got: {err}");
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
    fn detect_from_magic_matches_new_algos() {
        assert_eq!(
            detect_from_magic(&[0x04, 0x22, 0x4d, 0x18]),
            Some(Algo::Lz4)
        );
        assert_eq!(
            detect_from_magic(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]),
            Some(Algo::Xz)
        );
        assert_eq!(
            detect_from_magic(&[0xff, 0x06, 0x00, 0x00, 0x73, 0x4e, 0x61, 0x50, 0x70, 0x59]),
            Some(Algo::Snappy)
        );
    }

    #[test]
    fn detect_from_magic_recognises_zlib_cmf_flg() {
        // 0x78 0x9c is zlib's default level header; 0x78 0x01 is no-compression;
        // 0x78 0xda is best. All divisible by 31 when packed BE.
        assert_eq!(detect_from_magic(&[0x78, 0x9c]), Some(Algo::Zlib));
        assert_eq!(detect_from_magic(&[0x78, 0x01]), Some(Algo::Zlib));
        assert_eq!(detect_from_magic(&[0x78, 0xda]), Some(Algo::Zlib));
        // 0x78 0x00: (0x78*256 + 0) % 31 = 30628 % 31 = 0 → matches. Fine, any
        // CMF=0x78 + valid FLG is accepted by design.
        // Negative: second byte that doesn't pass the CMF/FLG check.
        assert_eq!(detect_from_magic(&[0x78, 0x02]), None);
        // First byte not 0x78 — no match.
        assert_eq!(detect_from_magic(&[0x77, 0x9c]), None);
    }

    #[test]
    fn levelless_algos_have_zero_range() {
        assert_eq!(Algo::Lz4.level_range(), (0, 0));
        assert_eq!(Algo::Snappy.level_range(), (0, 0));
        assert!(Algo::Lz4.is_levelless());
        assert!(Algo::Snappy.is_levelless());
        assert!(!Algo::Gzip.is_levelless());
        assert!(!Algo::Xz.is_levelless());
        assert!(!Algo::Zlib.is_levelless());
    }

    #[test]
    fn new_algos_parse_and_canonicalize() {
        assert_eq!(parse_algo("lz4").unwrap(), Algo::Lz4);
        assert_eq!(parse_algo("lz").unwrap(), Algo::Lz4);
        assert_eq!(parse_algo("xz").unwrap(), Algo::Xz);
        assert_eq!(parse_algo("lzma").unwrap(), Algo::Xz);
        assert_eq!(parse_algo("snappy").unwrap(), Algo::Snappy);
        assert_eq!(parse_algo("snap").unwrap(), Algo::Snappy);
        assert_eq!(parse_algo("sz").unwrap(), Algo::Snappy);
        assert_eq!(parse_algo("zlib").unwrap(), Algo::Zlib);
        assert_eq!(parse_algo("zl").unwrap(), Algo::Zlib);
    }

    #[test]
    fn round_trip_all_new_algos() {
        let payload = b"the quick brown fox jumps over the lazy dog";
        for algo in [Algo::Lz4, Algo::Xz, Algo::Snappy, Algo::Zlib] {
            let mut compressed = Vec::new();
            let reader: Box<dyn std::io::Read> = Box::new(std::io::Cursor::new(payload.to_vec()));
            compress(algo, algo.default_level(), reader, &mut compressed).unwrap();
            assert!(!compressed.is_empty(), "{:?} produced empty output", algo);

            let mut decompressed = Vec::new();
            let reader: Box<dyn std::io::Read> =
                Box::new(std::io::Cursor::new(compressed.clone()));
            decompress(algo, reader, &mut decompressed).unwrap();
            assert_eq!(
                decompressed, payload,
                "{:?} round-trip mismatch",
                algo
            );
        }
    }

    #[test]
    fn algo_all_has_five_variants() {
        assert_eq!(Algo::ALL.len(), 9);
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

    #[test]
    fn parse_level_numbers() {
        assert_eq!(parse_level("0").unwrap(), Level::Num(0));
        assert_eq!(parse_level("6").unwrap(), Level::Num(6));
        assert_eq!(parse_level("22").unwrap(), Level::Num(22));
    }

    #[test]
    fn parse_level_words_case_insensitive() {
        assert_eq!(parse_level("fastest").unwrap(), Level::Word(LevelWord::Fastest));
        assert_eq!(parse_level("FAST").unwrap(), Level::Word(LevelWord::Fast));
        assert_eq!(parse_level("Default").unwrap(), Level::Word(LevelWord::Default));
        assert_eq!(parse_level("good").unwrap(), Level::Word(LevelWord::Good));
        assert_eq!(parse_level("best").unwrap(), Level::Word(LevelWord::Best));
    }

    #[test]
    fn parse_level_unknown_word_errors() {
        let err = parse_level("fastestish").unwrap_err().to_string();
        assert!(err.contains("fastestish"), "got: {err}");
        assert!(err.contains("fastest"), "got: {err}");
    }

    #[test]
    fn parse_level_garbage_errors() {
        let err = parse_level("1.5").unwrap_err().to_string();
        assert!(err.contains("1.5"), "got: {err}");
    }

    #[test]
    fn resolve_word_levels_per_algorithm() {
        // Spot-check a few entries from the word-to-native table.
        assert_eq!(resolve_native_level(Algo::Gzip, Level::Word(LevelWord::Fastest)).unwrap(), 1);
        assert_eq!(resolve_native_level(Algo::Gzip, Level::Word(LevelWord::Best)).unwrap(), 9);
        assert_eq!(resolve_native_level(Algo::Zstd, Level::Word(LevelWord::Default)).unwrap(), 3);
        assert_eq!(resolve_native_level(Algo::Zstd, Level::Word(LevelWord::Best)).unwrap(), 22);
        assert_eq!(resolve_native_level(Algo::Brotli, Level::Word(LevelWord::Default)).unwrap(), 4);
        assert_eq!(resolve_native_level(Algo::Brotli, Level::Word(LevelWord::Best)).unwrap(), 11);
        assert_eq!(resolve_native_level(Algo::Bzip2, Level::Word(LevelWord::Default)).unwrap(), 6);
    }

    #[test]
    fn resolve_numeric_level_in_range() {
        assert_eq!(resolve_native_level(Algo::Gzip, Level::Num(5)).unwrap(), 5);
        assert_eq!(resolve_native_level(Algo::Zstd, Level::Num(22)).unwrap(), 22);
        assert_eq!(resolve_native_level(Algo::Brotli, Level::Num(0)).unwrap(), 0);
    }

    #[test]
    fn resolve_numeric_level_out_of_range_errors() {
        let err = resolve_native_level(Algo::Gzip, Level::Num(10)).unwrap_err().to_string();
        assert!(err.contains("10"), "got: {err}");
        assert!(err.contains("gzip"), "got: {err}");
        assert!(err.contains("0-9"), "got: {err}");

        let err = resolve_native_level(Algo::Zstd, Level::Num(23)).unwrap_err().to_string();
        assert!(err.contains("zstd"), "got: {err}");
        assert!(err.contains("1-22"), "got: {err}");
    }

    #[test]
    fn run_compress_file_then_decompress_roundtrips() {
        use clap::Parser;

        let tmp_in = std::env::temp_dir().join(format!(
            "recon-compress-in-{}.bin",
            std::process::id()
        ));
        let tmp_enc = std::env::temp_dir().join(format!(
            "recon-compress-enc-{}.gz",
            std::process::id()
        ));
        let tmp_dec = std::env::temp_dir().join(format!(
            "recon-compress-dec-{}.bin",
            std::process::id()
        ));

        std::fs::write(&tmp_in, b"hello compression").unwrap();

        // Compress.
        let args = Args::try_parse_from([
            "recon",
            "--compress",
            "gzip",
            "-o",
            tmp_enc.to_str().unwrap(),
            tmp_in.to_str().unwrap(),
        ]).unwrap();
        run(&args).unwrap();

        // Confirm output starts with gzip magic.
        let compressed = std::fs::read(&tmp_enc).unwrap();
        assert_eq!(&compressed[..2], &[0x1f, 0x8b]);

        // Decompress (auto-detect) back to tmp_dec.
        let args = Args::try_parse_from([
            "recon",
            "--decompress",
            "-o",
            tmp_dec.to_str().unwrap(),
            tmp_enc.to_str().unwrap(),
        ]).unwrap();
        run(&args).unwrap();

        let got = std::fs::read(&tmp_dec).unwrap();
        assert_eq!(got, b"hello compression");

        let _ = std::fs::remove_file(&tmp_in);
        let _ = std::fs::remove_file(&tmp_enc);
        let _ = std::fs::remove_file(&tmp_dec);
    }

    fn round_trip(algo: Algo, input: &[u8]) -> Vec<u8> {
        // Compress into a Vec<u8>.
        let mut encoded = Vec::new();
        let source: Box<dyn Read> = Box::new(std::io::Cursor::new(input.to_vec()));
        let level = algo.default_level();
        compress(algo, level, source, &mut encoded).unwrap();

        // Decompress the result.
        let mut decoded = Vec::new();
        let source: Box<dyn Read> = Box::new(std::io::Cursor::new(encoded));
        decompress(algo, source, &mut decoded).unwrap();

        decoded
    }

    #[test]
    fn round_trip_gzip_short_string() {
        let got = round_trip(Algo::Gzip, b"hello recon");
        assert_eq!(got, b"hello recon");
    }

    #[test]
    fn round_trip_deflate_short_string() {
        let got = round_trip(Algo::Deflate, b"hello recon");
        assert_eq!(got, b"hello recon");
    }

    #[test]
    fn round_trip_zstd_short_string() {
        let got = round_trip(Algo::Zstd, b"hello recon");
        assert_eq!(got, b"hello recon");
    }

    #[test]
    fn round_trip_brotli_short_string() {
        let got = round_trip(Algo::Brotli, b"hello recon");
        assert_eq!(got, b"hello recon");
    }

    #[test]
    fn round_trip_bzip2_short_string() {
        let got = round_trip(Algo::Bzip2, b"hello recon");
        assert_eq!(got, b"hello recon");
    }

    #[test]
    fn round_trip_gzip_empty() {
        let got = round_trip(Algo::Gzip, b"");
        assert_eq!(got, b"");
    }

    #[test]
    fn round_trip_zstd_large_buffer() {
        // 1 MiB of a repeating pattern — verifies streaming chunks work.
        let input: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
        let got = round_trip(Algo::Zstd, &input);
        assert_eq!(got.len(), input.len());
        assert_eq!(got, input);
    }

    #[test]
    fn compressed_gzip_has_magic_bytes() {
        let mut encoded = Vec::new();
        let source: Box<dyn Read> = Box::new(std::io::Cursor::new(b"hello".to_vec()));
        compress(Algo::Gzip, 6, source, &mut encoded).unwrap();
        assert!(encoded.len() >= 2);
        assert_eq!(&encoded[..2], &[0x1f, 0x8b]);
    }

    #[test]
    fn compressed_zstd_has_magic_bytes() {
        let mut encoded = Vec::new();
        let source: Box<dyn Read> = Box::new(std::io::Cursor::new(b"hello".to_vec()));
        compress(Algo::Zstd, 3, source, &mut encoded).unwrap();
        assert!(encoded.len() >= 4);
        assert_eq!(&encoded[..4], &[0x28, 0xb5, 0x2f, 0xfd]);
    }

    #[test]
    fn compressed_bzip2_has_magic_bytes() {
        let mut encoded = Vec::new();
        let source: Box<dyn Read> = Box::new(std::io::Cursor::new(b"hello".to_vec()));
        compress(Algo::Bzip2, 6, source, &mut encoded).unwrap();
        assert!(encoded.len() >= 3);
        assert_eq!(&encoded[..3], b"BZh");
    }
}
