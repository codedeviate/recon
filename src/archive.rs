//! `--archive DEST FILE…` / `--extract SRC [-o DIR]` archive tools.
//!
//! Unified flags with extension-based format detection:
//!   .zip                 → zip
//!   .tar                 → uncompressed tar
//!   .tar.gz  / .tgz      → tar + gzip
//!   .tar.xz  / .txz      → tar + xz
//!   .tar.bz2 / .tbz2     → tar + bzip2
//!
//! For `--extract`, format is inferred from the extension first, then
//! magic-byte sniffing as a fallback (so `.dat` that's actually a ZIP
//! still works).

use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::cli::Args;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    Tar,
    TarGz,
    TarXz,
    TarBz2,
}

impl Format {
    pub fn label(&self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::Tar => "tar",
            Format::TarGz => "tar.gz",
            Format::TarXz => "tar.xz",
            Format::TarBz2 => "tar.bz2",
        }
    }
}

/// Detect archive format from a path's extension. Returns `None` when
/// the extension isn't recognised.
pub fn detect_from_path(path: &Path) -> Option<Format> {
    let s = path.to_string_lossy().to_ascii_lowercase();
    if s.ends_with(".zip") {
        Some(Format::Zip)
    } else if s.ends_with(".tar.gz") || s.ends_with(".tgz") {
        Some(Format::TarGz)
    } else if s.ends_with(".tar.xz") || s.ends_with(".txz") {
        Some(Format::TarXz)
    } else if s.ends_with(".tar.bz2") || s.ends_with(".tbz2") {
        Some(Format::TarBz2)
    } else if s.ends_with(".tar") {
        Some(Format::Tar)
    } else {
        None
    }
}

/// Detect from the first bytes of an already-opened file. Used for
/// `--extract` when the extension is missing or misleading.
pub fn detect_from_magic(head: &[u8]) -> Option<Format> {
    if head.len() >= 4 && (&head[..4] == b"PK\x03\x04" || &head[..4] == b"PK\x05\x06") {
        return Some(Format::Zip);
    }
    if head.len() >= 2 && head[0] == 0x1f && head[1] == 0x8b {
        return Some(Format::TarGz);
    }
    if head.len() >= 6 && head[..6] == [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] {
        return Some(Format::TarXz);
    }
    if head.len() >= 3 && head[..3] == [0x42, 0x5a, 0x68] {
        return Some(Format::TarBz2);
    }
    // ustar magic at offset 257 (tar header)
    if head.len() >= 265 && &head[257..262] == b"ustar" {
        return Some(Format::Tar);
    }
    None
}

/// Entry point for `--archive DEST FILE…`. Sources are `args.script_args`
/// via the argv pre-split in `Args::parse_with_script_split` (which also
/// handles `--archive`).
pub fn run_archive_cli(args: &Args) -> Result<()> {
    let dest = args
        .archive
        .as_deref()
        .ok_or_else(|| anyhow!("internal: --archive missing"))?;
    let format = detect_from_path(dest).ok_or_else(|| {
        anyhow!(
            "--archive: can't infer format from '{}'. Supported extensions: \
             .zip .tar .tar.gz/.tgz .tar.xz/.txz .tar.bz2/.tbz2",
            dest.display()
        )
    })?;
    let sources: Vec<PathBuf> = args
        .script_args
        .iter()
        .map(PathBuf::from)
        .collect();
    if sources.is_empty() {
        return Err(anyhow!("--archive: no input files. Usage: --archive DEST FILE [FILE ...]"));
    }

    let count = create(dest, &sources, format)?;
    let size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    println!("archive: {} ({})", dest.display(), format.label());
    println!("  {} file{}, {}", count, if count == 1 { "" } else { "s" }, format_size(size));
    Ok(())
}

/// Entry point for `--extract SRC [-o DIR]`.
pub fn run_extract_cli(args: &Args) -> Result<()> {
    let src = args
        .extract
        .as_deref()
        .ok_or_else(|| anyhow!("internal: --extract missing"))?;
    let dest_dir = args
        .output
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("--extract: create output dir {}", dest_dir.display()))?;

    // Extension first, then magic-byte sniff.
    let format = match detect_from_path(src) {
        Some(f) => f,
        None => {
            let mut head = [0u8; 512];
            let mut f = File::open(src)
                .with_context(|| format!("--extract: open {}", src.display()))?;
            let n = f.read(&mut head).unwrap_or(0);
            detect_from_magic(&head[..n]).ok_or_else(|| {
                anyhow!(
                    "--extract: can't infer format of '{}' from extension or magic bytes",
                    src.display()
                )
            })?
        }
    };

    let count = extract(src, &dest_dir, format)?;
    println!("extracted: {} ({})", dest_dir.display(), format.label());
    println!("  {} file{}", count, if count == 1 { "" } else { "s" });
    Ok(())
}

// ── Create ────────────────────────────────────────────────────────────────

pub fn create(dest: &Path, sources: &[PathBuf], format: Format) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    match format {
        Format::Zip => create_zip(dest, sources),
        Format::Tar => create_tar(dest, sources, TarCompression::None),
        Format::TarGz => create_tar(dest, sources, TarCompression::Gzip),
        Format::TarXz => create_tar(dest, sources, TarCompression::Xz),
        Format::TarBz2 => create_tar(dest, sources, TarCompression::Bzip2),
    }
}

enum TarCompression {
    None,
    Gzip,
    Xz,
    Bzip2,
}

fn create_tar(dest: &Path, sources: &[PathBuf], compression: TarCompression) -> Result<u64> {
    let file = File::create(dest)
        .with_context(|| format!("create {}", dest.display()))?;
    let writer: Box<dyn Write> = match compression {
        TarCompression::None => Box::new(file),
        TarCompression::Gzip => Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::default(),
        )),
        TarCompression::Xz => Box::new(xz2::write::XzEncoder::new(file, 6)),
        TarCompression::Bzip2 => Box::new(bzip2::write::BzEncoder::new(
            file,
            bzip2::Compression::default(),
        )),
    };
    let mut builder = tar::Builder::new(writer);
    let mut count = 0u64;
    for src in sources {
        count += tar_append(&mut builder, src)?;
    }
    // Finish writes the tar trailer; drop consumes the builder and flushes.
    builder
        .finish()
        .with_context(|| format!("finalise tar {}", dest.display()))?;
    drop(builder);
    Ok(count)
}

fn tar_append<W: Write>(builder: &mut tar::Builder<W>, src: &Path) -> Result<u64> {
    let meta = std::fs::metadata(src).with_context(|| format!("stat {}", src.display()))?;
    // Use the source path's file_name as the in-archive path, not the full
    // filesystem path. Matches user expectation from `tar cf` / `zip`.
    let archive_name = src
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(src));
    if meta.is_dir() {
        builder
            .append_dir_all(&archive_name, src)
            .with_context(|| format!("append dir {}", src.display()))?;
        let mut n = 0u64;
        for entry in walkdir::walk(src)? {
            if entry.is_file() {
                n += 1;
            }
        }
        Ok(n)
    } else {
        let mut f = File::open(src)
            .with_context(|| format!("open {}", src.display()))?;
        builder
            .append_file(&archive_name, &mut f)
            .with_context(|| format!("append file {}", src.display()))?;
        Ok(1)
    }
}

fn create_zip(dest: &Path, sources: &[PathBuf]) -> Result<u64> {
    let file = File::create(dest)
        .with_context(|| format!("create {}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut count = 0u64;
    for src in sources {
        let meta = std::fs::metadata(src).with_context(|| format!("stat {}", src.display()))?;
        if meta.is_dir() {
            let prefix = src
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            for entry in walkdir::walk(src)? {
                if entry.is_dir {
                    let rel = relative_under(src, &entry.path, &prefix);
                    zip.add_directory(rel.to_string_lossy(), options)
                        .map_err(|e| anyhow!("zip add_directory: {e}"))?;
                } else {
                    let rel = relative_under(src, &entry.path, &prefix);
                    zip.start_file(rel.to_string_lossy(), options)
                        .map_err(|e| anyhow!("zip start_file: {e}"))?;
                    let mut f = File::open(&entry.path)
                        .with_context(|| format!("open {}", entry.path.display()))?;
                    std::io::copy(&mut f, &mut zip)
                        .with_context(|| format!("write {}", entry.path.display()))?;
                    count += 1;
                }
            }
        } else {
            let name = src
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| src.display().to_string());
            zip.start_file(name, options)
                .map_err(|e| anyhow!("zip start_file: {e}"))?;
            let mut f = File::open(src)
                .with_context(|| format!("open {}", src.display()))?;
            std::io::copy(&mut f, &mut zip)
                .with_context(|| format!("write {}", src.display()))?;
            count += 1;
        }
    }
    zip.finish().map_err(|e| anyhow!("zip finish: {e}"))?;
    Ok(count)
}

/// Build an archive-relative path for a file under a walked directory.
/// `root` is the directory being archived (e.g. `/home/foo/src`); `path`
/// is the current file under it; `archive_prefix` is the directory name
/// that should appear at the archive root (e.g. `src`).
fn relative_under(root: &Path, path: &Path, archive_prefix: &Path) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(rest) if rest.as_os_str().is_empty() => archive_prefix.to_path_buf(),
        Ok(rest) => archive_prefix.join(rest),
        Err(_) => PathBuf::from(path.file_name().unwrap_or_default()),
    }
}

// ── Extract ───────────────────────────────────────────────────────────────

pub fn extract(src: &Path, dest_dir: &Path, format: Format) -> Result<u64> {
    match format {
        Format::Zip => extract_zip(src, dest_dir),
        Format::Tar => extract_tar(src, dest_dir, TarCompression::None),
        Format::TarGz => extract_tar(src, dest_dir, TarCompression::Gzip),
        Format::TarXz => extract_tar(src, dest_dir, TarCompression::Xz),
        Format::TarBz2 => extract_tar(src, dest_dir, TarCompression::Bzip2),
    }
}

fn extract_tar(src: &Path, dest: &Path, compression: TarCompression) -> Result<u64> {
    let file = File::open(src).with_context(|| format!("open {}", src.display()))?;
    let reader: Box<dyn Read> = match compression {
        TarCompression::None => Box::new(file),
        TarCompression::Gzip => Box::new(flate2::read::GzDecoder::new(file)),
        TarCompression::Xz => Box::new(xz2::read::XzDecoder::new(file)),
        TarCompression::Bzip2 => Box::new(bzip2::read::BzDecoder::new(file)),
    };
    let mut archive = tar::Archive::new(reader);
    let mut count = 0u64;
    for entry in archive.entries()? {
        let mut e = entry.context("tar entry")?;
        if e.header().entry_type().is_file() {
            count += 1;
        }
        e.unpack_in(dest)
            .with_context(|| format!("unpack into {}", dest.display()))?;
    }
    Ok(count)
}

fn extract_zip(src: &Path, dest: &Path) -> Result<u64> {
    let file = File::open(src).with_context(|| format!("open {}", src.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("zip open: {e}"))?;
    archive
        .extract(dest)
        .map_err(|e| anyhow!("zip extract: {e}"))?;
    // Count entries that are files (not directories).
    let mut count = 0u64;
    let file2 = File::open(src)?;
    let mut archive2 = zip::ZipArchive::new(file2).map_err(|e| anyhow!("zip reopen: {e}"))?;
    for i in 0..archive2.len() {
        let entry = archive2.by_index(i).map_err(|e| anyhow!("zip index: {e}"))?;
        if entry.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

// ── Small utilities ───────────────────────────────────────────────────────

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Tiny in-module directory walker. No new dep; just recursive `read_dir`.
mod walkdir {
    use anyhow::{Context, Result};
    use std::path::{Path, PathBuf};

    pub struct Entry {
        pub path: PathBuf,
        pub is_dir: bool,
    }

    impl Entry {
        pub fn is_file(&self) -> bool {
            !self.is_dir
        }
    }

    pub fn walk(root: &Path) -> Result<Vec<Entry>> {
        let mut out = Vec::new();
        push_recursive(root, &mut out)?;
        Ok(out)
    }

    fn push_recursive(path: &Path, out: &mut Vec<Entry>) -> Result<()> {
        let meta = std::fs::metadata(path)
            .with_context(|| format!("stat {}", path.display()))?;
        if meta.is_dir() {
            out.push(Entry {
                path: path.to_path_buf(),
                is_dir: true,
            });
            for entry in std::fs::read_dir(path)
                .with_context(|| format!("read_dir {}", path.display()))?
            {
                let entry = entry?;
                push_recursive(&entry.path(), out)?;
            }
        } else {
            out.push(Entry {
                path: path.to_path_buf(),
                is_dir: false,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_from_path_extensions() {
        assert_eq!(detect_from_path(Path::new("foo.zip")), Some(Format::Zip));
        assert_eq!(detect_from_path(Path::new("foo.tar")), Some(Format::Tar));
        assert_eq!(detect_from_path(Path::new("foo.tar.gz")), Some(Format::TarGz));
        assert_eq!(detect_from_path(Path::new("foo.tgz")), Some(Format::TarGz));
        assert_eq!(detect_from_path(Path::new("foo.tar.xz")), Some(Format::TarXz));
        assert_eq!(detect_from_path(Path::new("foo.txz")), Some(Format::TarXz));
        assert_eq!(detect_from_path(Path::new("foo.tar.bz2")), Some(Format::TarBz2));
        assert_eq!(detect_from_path(Path::new("foo.tbz2")), Some(Format::TarBz2));
        assert_eq!(detect_from_path(Path::new("foo.tar.ZIP")), Some(Format::Zip));
        assert_eq!(detect_from_path(Path::new("foo.unknown")), None);
        assert_eq!(detect_from_path(Path::new("")), None);
    }

    #[test]
    fn detect_from_magic_prefixes() {
        assert_eq!(
            detect_from_magic(b"PK\x03\x04some zip content"),
            Some(Format::Zip)
        );
        assert_eq!(
            detect_from_magic(&[0x1f, 0x8b, 0x08, 0x00]),
            Some(Format::TarGz)
        );
        assert_eq!(
            detect_from_magic(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]),
            Some(Format::TarXz)
        );
        assert_eq!(detect_from_magic(b"BZh91AY&"), Some(Format::TarBz2));
        assert_eq!(detect_from_magic(b"unknown"), None);
    }

    #[test]
    fn zip_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let src_file = dir.path().join("hello.txt");
        std::fs::write(&src_file, b"hi from zip").unwrap();
        let dest = dir.path().join("out.zip");
        let n = create(&dest, &[src_file], Format::Zip).unwrap();
        assert_eq!(n, 1);

        let out_dir = dir.path().join("extract");
        let n2 = extract(&dest, &out_dir, Format::Zip).unwrap();
        assert_eq!(n2, 1);
        let got = std::fs::read(out_dir.join("hello.txt")).unwrap();
        assert_eq!(got, b"hi from zip");
    }

    #[test]
    fn tar_gz_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let src_file = dir.path().join("greeting.txt");
        std::fs::write(&src_file, b"tar.gz works").unwrap();
        let dest = dir.path().join("out.tar.gz");
        let n = create(&dest, &[src_file], Format::TarGz).unwrap();
        assert_eq!(n, 1);

        let out_dir = dir.path().join("extract");
        std::fs::create_dir_all(&out_dir).unwrap();
        let n2 = extract(&dest, &out_dir, Format::TarGz).unwrap();
        assert_eq!(n2, 1);
        let got = std::fs::read(out_dir.join("greeting.txt")).unwrap();
        assert_eq!(got, b"tar.gz works");
    }

    #[test]
    fn tar_xz_and_bz2_round_trip() {
        for format in [Format::TarXz, Format::TarBz2] {
            let dir = tempfile::tempdir().unwrap();
            let src_file = dir.path().join("a.txt");
            std::fs::write(&src_file, format!("payload for {:?}", format).as_bytes()).unwrap();
            let ext = match format {
                Format::TarXz => "tar.xz",
                Format::TarBz2 => "tar.bz2",
                _ => unreachable!(),
            };
            let dest = dir.path().join(format!("out.{ext}"));
            create(&dest, &[src_file.clone()], format).unwrap();
            let out_dir = dir.path().join("extract");
            std::fs::create_dir_all(&out_dir).unwrap();
            extract(&dest, &out_dir, format).unwrap();
            let got = std::fs::read(out_dir.join("a.txt")).unwrap();
            assert_eq!(got, format!("payload for {:?}", format).as_bytes());
        }
    }

    #[test]
    fn format_size_ranges() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert!(format_size(5 * 1024 * 1024).starts_with("5.00 MB"));
    }
}
