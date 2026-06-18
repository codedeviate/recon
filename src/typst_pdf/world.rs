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

#[allow(dead_code)]
impl ReconWorld {
    /// Build a world from a complete typst source string plus any auxiliary
    /// files. `files` is empty during the spike; later tasks populate it with
    /// embedded image bytes.
    pub fn new(main_src: String, files: HashMap<FileId, Bytes>) -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
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
