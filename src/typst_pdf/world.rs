//! Minimal `typst::World` implementation for the embedded PDF engine.
//!
//! This is the de-risking spike (Task 1.1): it carries the bundled fonts, a
//! default `Library`, and a single synthetic main `Source`. Image/auxiliary
//! file support (the `files` map) is plumbed but unused at this stage — the
//! full Markdown→typst translator wires it up in later tasks.

use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

/// A self-contained `World` backed by bundled fonts and an in-memory source.
// `dead_code`: constructed by the CLI/translator landing in a later task.
#[allow(dead_code)]
pub struct ReconWorld {
    /// The standard library (default configuration).
    library: LazyHash<Library>,
    /// Metadata about all bundled fonts.
    book: LazyHash<FontBook>,
    /// The actual bundled font faces, indexed in parallel with `book`.
    fonts: Vec<Font>,
    /// The single main source file being compiled.
    main: Source,
    /// Auxiliary files (images, etc.), keyed by `FileId`. Empty for now.
    #[allow(dead_code)]
    files: HashMap<FileId, Bytes>,
}

/// Bundled IBM Plex Sans faces (OFL), the selectable proportional sans for
/// `--font 'IBM Plex Sans'`. typst's own `typst_assets::fonts()` ships only a
/// serif body (Libertinus Serif) and a monospace (DejaVu Sans Mono), so the
/// engine has no proportional sans to switch to without this. The license is
/// in `assets/fonts/LICENSE-IBMPlexSans.txt`.
const BUNDLED_FONTS: &[&[u8]] = &[
    include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-Italic.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-Bold.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-BoldItalic.ttf"),
];

#[allow(dead_code)]
impl ReconWorld {
    /// Build a world from a complete typst source string plus any auxiliary
    /// files. `files` is empty during the spike; later tasks populate it with
    /// embedded image bytes. `font_dirs` are extra directories (`--font-path`)
    /// scanned for fonts on top of typst's bundled set and recon's bundled
    /// IBM Plex Sans.
    pub fn new(main_src: String, files: HashMap<FileId, Bytes>, font_dirs: &[String]) -> Self {
        let mut fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        // recon's bundled proportional sans (selectable via `--font`).
        for data in BUNDLED_FONTS {
            fonts.extend(Font::iter(Bytes::new(*data)));
        }
        // User-supplied font directories (`--font-path`). Unreadable entries
        // and non-font files are skipped silently; a missing directory is
        // warned about by the CLI before rendering, not here.
        for dir in font_dirs {
            load_fonts_from_dir(std::path::Path::new(dir), &mut fonts);
        }
        let book = FontBook::from_fonts(&fonts);
        let main = Source::detached(main_src);
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main,
            files,
        }
    }
}

/// Recursively scan `dir` for font files and append every face to `fonts`.
///
/// Recognises `.ttf`, `.otf`, `.ttc`, and `.otc` (case-insensitive). A
/// collection file (`.ttc`/`.otc`) yields multiple faces via `Font::iter`.
/// Anything that fails to read or parse is skipped silently — a font
/// directory is a best-effort escape hatch, not a hard dependency.
fn load_fonts_from_dir(dir: &std::path::Path, fonts: &mut Vec<Font>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_fonts_from_dir(&path, fonts);
            continue;
        }
        let is_font = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc"))
            .unwrap_or(false);
        if !is_font {
            continue;
        }
        if let Ok(data) = std::fs::read(&path) {
            fonts.extend(Font::iter(Bytes::new(data)));
        }
    }
}

impl World for ReconWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // Returning `None` keeps PDF output deterministic (no embedded clock).
        None
    }
}
