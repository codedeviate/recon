use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Response;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::Args;
use crate::fail::FailMode;
use crate::metrics::RequestMetrics;

/// Where the processed response body is written.
///
/// `Writer` streams as it processes (default for stdout). `File` writes the
/// processed bytes to a path. `Editor` and `Clipboard` accumulate the full
/// output internally before dispatching, since both need the complete buffer.
pub enum BodySink<'a> {
    /// Write to the supplied writer (typically stdout).
    Writer(&'a mut dyn Write),
    /// Write to FILE.
    File(&'a std::path::Path),
    /// Buffer fully, write to a temp file, spawn the user's editor.
    Editor,
    /// Buffer fully, write to the system clipboard.
    Clipboard,
}

/// Wrap a writer with rate-limiting and/or speed-watchdog layers when
/// `--limit-rate` / `--speed-limit` are set. Layers compose:
/// `SpeedWatchWriter<RateLimitedWriter<Box<dyn Write + 'a>>>`. When
/// neither flag is set, the original writer passes through (still boxed
/// so the caller can use one type).
fn wrap_with_rate_control<'a>(
    writer: Box<dyn Write + 'a>,
    args: &Args,
) -> anyhow::Result<Box<dyn Write + 'a>> {
    let mut out: Box<dyn Write + 'a> = writer;
    if let Some(rate_str) = &args.limit_rate {
        let rate = crate::ratelimit::parse_rate(rate_str)?;
        out = Box::new(crate::ratelimit::RateLimitedWriter::new(out, rate));
    }
    if let Some(floor) = args.speed_limit {
        let window = std::time::Duration::from_secs(args.speed_time);
        out = Box::new(crate::ratelimit::SpeedWatchWriter::new(out, floor, window));
    }
    Ok(out)
}

/// Thin Write wrapper that tallies bytes written into a caller-provided counter.
/// Used to populate `metrics.size_download` while streaming the body.
struct CountingWriter<'a, W: Write> {
    inner: W,
    count: &'a mut u64,
}

impl<'a, W: Write> Write for CountingWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        *self.count += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Destination for the "stdout-bound" portion of the response: the body and
/// the stdout-bound header block (i.e., when `-i`, `--full`, `--LHEAD`, or
/// `-I` routes headers to stdout instead of stderr). Diagnostic headers at
/// `-v` still go to stderr via the real stderr handle, unchanged.
pub enum StdoutSink {
    /// Write to the process's real stdout.
    Stdout,
    /// Write to an in-memory buffer (used by `--editor`). After
    /// `write_response` returns the caller can read the buffer and route it
    /// wherever it needs to go.
    Buffer(Vec<u8>),
    /// Write to both stdout and an in-memory buffer (used by `--editor -vv`).
    Tee(Vec<u8>),
}

impl StdoutSink {
    fn writer(&mut self) -> Box<dyn Write + '_> {
        match self {
            StdoutSink::Stdout => Box::new(io::stdout()),
            StdoutSink::Buffer(buf) => Box::new(buf),
            StdoutSink::Tee(buf) => Box::new(TeeWriter {
                a: io::stdout(),
                b: buf,
            }),
        }
    }

    /// Consume the sink and return the captured bytes, if any.
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            StdoutSink::Stdout => None,
            StdoutSink::Buffer(b) | StdoutSink::Tee(b) => Some(b),
        }
    }
}

struct TeeWriter<'a, A: Write> {
    a: A,
    b: &'a mut Vec<u8>,
}

impl<'a, A: Write> Write for TeeWriter<'a, A> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.a.write(buf)?;
        self.b.extend_from_slice(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.a.flush()?;
        // Vec<u8>'s Write impl never errors on flush.
        Ok(())
    }
}

/// Parse a byte size with optional K/M/G suffix (1K = 1024). Case
/// insensitive. Accepts plain `4096` / `4K` / `2.5M` / `1g`.
pub fn parse_size_with_suffix(s: &str) -> anyhow::Result<u64> {
    let t = s.trim();
    if t.is_empty() {
        anyhow::bail!("empty size");
    }
    let (num_part, mul): (&str, u64) = match t.chars().last().unwrap() {
        'K' | 'k' => (&t[..t.len() - 1], 1024),
        'M' | 'm' => (&t[..t.len() - 1], 1024 * 1024),
        'G' | 'g' => (&t[..t.len() - 1], 1024 * 1024 * 1024),
        _ => (t, 1),
    };
    let n: f64 = num_part.parse().map_err(|_| anyhow::anyhow!("bad number: {num_part}"))?;
    if n < 0.0 {
        anyhow::bail!("size must be non-negative");
    }
    Ok((n * mul as f64) as u64)
}

pub fn write_response(response: Response, args: &Args, metrics: &mut RequestMetrics) -> Result<()> {
    let mut sink = StdoutSink::Stdout;
    write_response_to(response, args, &mut sink, metrics)
}

/// Same as `write_response`, but the stdout-bound bytes go through `sink`.
pub fn write_response_to(
    mut response: Response,
    args: &Args,
    sink: &mut StdoutSink,
    metrics: &mut RequestMetrics,
) -> Result<()> {
    // Snapshot values needed after the body is consumed (for --xattr).
    let response_url_str = response.url().to_string();
    let response_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // --max-filesize: abort before streaming body when the server
    // announced a Content-Length over the user's limit.
    if let Some(raw) = args.max_filesize.as_deref() {
        let limit = parse_size_with_suffix(raw)
            .map_err(|e| anyhow::anyhow!("--max-filesize: {e}"))?;
        if let Some(v) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
            if let Ok(len) = v.to_str().unwrap_or("0").parse::<u64>() {
                if len > limit {
                    anyhow::bail!(
                        "--max-filesize: server announced {len} bytes > limit {limit}"
                    );
                }
            }
        }
    }

    // --dump-header: write response headers to FILE. Happens before
    // the body so the file is complete even if the body stream errors.
    if let Some(path) = args.dump_header.as_ref() {
        use std::io::Write;
        let mut f = std::fs::File::create(path)
            .with_context(|| format!("--dump-header: create {}", path.display()))?;
        writeln!(
            f,
            "HTTP/{:?} {}",
            response.version(),
            response.status()
        )?;
        for (k, v) in response.headers() {
            writeln!(f, "{}: {}", k, v.to_str().unwrap_or(""))?;
        }
    }

    // --etag-save: persist the response ETag for a future
    // --etag-compare round-trip.
    if let Some(path) = args.etag_save.as_ref() {
        if let Some(etag) = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
        {
            std::fs::write(path, etag)
                .with_context(|| format!("--etag-save: write {}", path.display()))?;
        }
    }

    // --no-clobber: refuse to overwrite the -o target.
    if args.no_clobber {
        if let Some(p) = args.output.as_ref() {
            if p.exists() {
                anyhow::bail!(
                    "--no-clobber: refusing to overwrite existing file {}",
                    p.display()
                );
            }
        }
    }

    let status = response.status();

    if args.status_only {
        let mut out = sink.writer();
        writeln!(out, "{}", status.as_u16())?;
        metrics.response_end = Some(std::time::Instant::now());
        return Ok(());
    }

    let print_headers = args.verbose >= 1
        || args.include_headers
        || args.head_only
        || args.lhead
        || args.full;
    let headers_to_stdout =
        args.include_headers || args.head_only || args.lhead || args.full;
    let print_body = !args.head_only || args.full;

    if print_headers {
        let status_str = format!(
            "HTTP/{} {} {}",
            match response.version() {
                reqwest::Version::HTTP_10 => "1.0",
                reqwest::Version::HTTP_11 => "1.1",
                reqwest::Version::HTTP_2 => "2",
                reqwest::Version::HTTP_3 => "3",
                _ => "?",
            },
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
        let colored_status = if status.is_success() {
            status_str.green().to_string()
        } else if status.is_redirection() {
            status_str.yellow().to_string()
        } else {
            status_str.red().to_string()
        };

        if headers_to_stdout {
            let mut out = sink.writer();
            if args.lhead {
                writeln!(out, "* {}", response.url())?;
            }
            writeln!(out, "< {colored_status}")?;
            for (name, value) in response.headers() {
                writeln!(out, "< {}: {}", name, value.to_str().unwrap_or("?"))?;
            }
            writeln!(out, "<")?;
        } else {
            let mut err = io::stderr();
            if args.lhead {
                writeln!(err, "* {}", response.url())?;
            }
            writeln!(err, "< {colored_status}")?;
            for (name, value) in response.headers() {
                writeln!(err, "< {}: {}", name, value.to_str().unwrap_or("?"))?;
            }
            writeln!(err, "<")?;
        }
    }

    let fail_mode = FailMode::from_args(args);
    let is_error = status.as_u16() >= 400;

    // -f: abort BEFORE body write
    if fail_mode == FailMode::OnError && is_error {
        metrics.response_end = Some(std::time::Instant::now());
        return Err(anyhow!(
            "HTTP error {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }

    let cd_filename = if args.remote_header_name {
        response
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(crate::remote_name::filename_from_content_disposition)
    } else {
        None
    };
    let final_path = resolve_output_path(args, response.url().as_str(), cd_filename.as_deref())?;

    // Capture Last-Modified now before the body consumes `response`.
    let last_modified_ts: Option<i64> = if args.remote_time {
        response
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_http_date)
    } else {
        None
    };

    // Run the body-write inside an IIFE so we can stamp response_end even
    // when the body I/O fails. Without this, a mid-transfer error would leave
    // response_end = None and `-w` would report time_total = 0.
    // Resolve the target output charset (--output-charset or --to-utf8
    // alias). When set, we buffer the body to transcode. Otherwise we
    // keep the zero-copy streaming path for large downloads.
    let output_charset_label: Option<String> = if let Some(c) = &args.output_charset {
        Some(c.clone())
    } else if args.to_utf8 {
        Some("utf-8".to_string())
    } else {
        None
    };

    let body_io_result: Result<()> = (|| -> Result<()> {
        if !print_body {
            return Ok(());
        }
        let content_type_str = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if args.prettify || output_charset_label.is_some()
            || args.editor.is_some() || args.to_clipboard || args.render {
            // Buffer + transcode path. Used when the user asked for
            // prettification (which needs a String anyway), explicit charset
            // conversion, editor dispatch, or clipboard output.
            let raw = response.bytes().context("Failed to read response body")?;

            let mut writer_holder;
            let body_sink = if args.editor.is_some() {
                BodySink::Editor
            } else if args.to_clipboard {
                BodySink::Clipboard
            } else if let Some(p) = final_path.as_deref() {
                BodySink::File(p)
            } else {
                writer_holder = sink.writer();
                BodySink::Writer(&mut *writer_holder)
            };

            let bytes_written = write_processed_body(
                args,
                &raw,
                &content_type_str,
                output_charset_label.as_deref(),
                body_sink,
            )?;
            metrics.size_download = bytes_written;
        } else {
            let content_length = response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            if let Some(path) = &final_path {
                if args.create_dirs {
                    ensure_parent_dir(path)?;
                }
                let file = File::create(path)?;
                let wrapped = wrap_with_rate_control(Box::new(file), args)?;
                let mut cw = CountingWriter { inner: wrapped, count: &mut metrics.size_download };
                if (args.progress || args.progress_bar) && !args.no_progress_meter {
                    let pb = make_progress_bar(content_length, args.progress_bar);
                    copy_with_progress(&mut response, &mut cw, &pb)?;
                    pb.finish_and_clear();
                } else {
                    io::copy(&mut response, &mut cw)?;
                }
                if !args.silent {
                    eprintln!("Saved to {}", path.display());
                }
            } else {
                let writer: Box<dyn Write> = Box::new(sink.writer());
                let wrapped = wrap_with_rate_control(writer, args)?;
                let mut cw = CountingWriter { inner: wrapped, count: &mut metrics.size_download };
                io::copy(&mut response, &mut cw)?;
            }
        }
        Ok(())
    })();

    // All exit paths below represent "body/headers done": stamp response_end once.
    metrics.response_end = Some(std::time::Instant::now());

    // --remove-on-error: unlink the -o target if the body stream errored.
    if body_io_result.is_err() && args.remove_on_error {
        if let Some(path) = &final_path {
            let _ = std::fs::remove_file(path);
            if !args.silent {
                eprintln!("--remove-on-error: deleted partial output {}", path.display());
            }
        }
    }
    body_io_result?;

    // --create-file-mode: chmod the saved file (Unix only).
    if let Some(raw) = args.create_file_mode.as_deref() {
        if let Some(path) = &final_path {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = u32::from_str_radix(raw, 8).with_context(|| {
                    format!("--create-file-mode: '{raw}' is not octal (e.g. 600)")
                })?;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
                    .with_context(|| {
                        format!("--create-file-mode: chmod {}", path.display())
                    })?;
            }
            #[cfg(not(unix))]
            {
                let _ = (raw, path);
                if !args.silent {
                    eprintln!("--create-file-mode: ignored on non-Unix platforms");
                }
            }
        }
    }

    // --remote-time: apply Last-Modified to the saved file
    if let (Some(path), Some(mtime)) = (&final_path, last_modified_ts) {
        let ft = filetime::FileTime::from_unix_time(mtime, 0);
        let _ = filetime::set_file_mtime(path, ft); // silent no-op on failure
    }

    // --xattr: write URL + MIME type into extended attributes.
    // Matches curl's user.xdg.origin.url / user.mime_type keys.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if args.xattr {
        if let Some(path) = &final_path {
            let _ = xattr::set(path, "user.xdg.origin.url", response_url_str.as_bytes());
            if let Some(ct) = response_content_type.as_deref() {
                let _ = xattr::set(path, "user.mime_type", ct.as_bytes());
            }
        }
    }

    // --fail-with-body: body written above, NOW return error so process exits non-zero
    if fail_mode == FailMode::OnErrorKeepBody && is_error {
        return Err(anyhow!(
            "HTTP error {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }

    Ok(())
}

pub(crate) fn make_progress_bar(total: Option<u64>, hash_style: bool) -> ProgressBar {
    match total {
        Some(len) => {
            let pb = ProgressBar::new(len);
            if hash_style {
                // -# / --progress-bar: curl-parity hash bar
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{bytes}/{total_bytes} [{bar:40.#->}] {bytes_per_sec} eta {eta}")
                        .unwrap()
                        .progress_chars("##-"),
                );
            } else {
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{bytes}/{total_bytes} [{bar:40}] {bytes_per_sec} eta {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
            }
            pb
        }
        None => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} {bytes} downloaded ({bytes_per_sec})")
                    .unwrap(),
            );
            pb
        }
    }
}

pub(crate) fn copy_with_progress(
    src: &mut impl io::Read,
    dst: &mut impl io::Write,
    pb: &ProgressBar,
) -> Result<()> {
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        pb.inc(n as u64);
    }
    Ok(())
}

/// Resolve the final output path for the response, applying curl's precedence rules:
/// 1. If `-o <path>` is set, use it. Prefix with `--output-dir` if set.
/// 2. Else if `-O` is set (unit-test path only; main.rs pre-resolves -O in real runs),
///    derive basename from Content-Disposition if `--remote-header-name` + `header_filename`
///    set, otherwise from URL path. Prefix with `--output-dir` if set.
/// 3. Else return None (output goes to stdout).
pub fn resolve_output_path(
    args: &Args,
    url: &str,
    header_filename: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    if let Some(explicit) = &args.output {
        let final_path = match &args.output_dir {
            Some(dir) => dir.join(explicit),
            None => explicit.clone(),
        };
        return Ok(Some(final_path));
    }

    if args.remote_name {
        // NOTE: in real runs main.rs pre-resolves -O into args.output via
        // util::filename_from_url, so this branch is only hit by unit tests
        // and by callers that bypass that pre-processing. Keep the shape
        // here so Task 9 can wire remote_header_name through.
        let basename = if args.remote_header_name {
            header_filename
                .map(str::to_string)
                .unwrap_or_else(|| basename_from_url(url))
        } else {
            basename_from_url(url)
        };
        let final_path = match &args.output_dir {
            Some(dir) => dir.join(&basename),
            None => PathBuf::from(&basename),
        };
        return Ok(Some(final_path));
    }

    Ok(None)
}

/// Process a fully-buffered body through the user's post-fetch pipeline:
/// optional charset transcode → optional prettify → dispatch to `sink`.
/// Returns the byte count written (used for the `size_download` metric).
///
/// Used by both the HTTP response path (in `write_response_to`) and the
/// `--stdin` mode dispatched from `main.rs`.
pub fn write_processed_body(
    args: &crate::cli::Args,
    raw: &[u8],
    content_type: &str,
    output_charset_label: Option<&str>,
    sink: BodySink<'_>,
) -> anyhow::Result<u64> {
    // 1. Optional charset transcode
    let body_bytes: Vec<u8> = if let Some(target_label) = output_charset_label {
        let target = crate::text_encoding::resolve(target_label)
            .with_context(|| format!("--output-charset: {target_label}"))?;
        let source_label = resolve_source_charset(args, content_type, raw);
        let source = crate::text_encoding::resolve(&source_label)
            .unwrap_or(encoding_rs::UTF_8);
        if source == target {
            raw.to_vec()
        } else {
            let r = crate::text_encoding::transcode(raw, source, target);
            if r.had_unmappable && !args.silent {
                eprintln!(
                    "! response body: one or more characters not representable in {} — substituted with '?'",
                    target.name()
                );
            }
            r.bytes
        }
    } else {
        raw.to_vec()
    };

    // 2. Optional prettify → produce processed_bytes
    let processed_bytes: Vec<u8> = if args.prettify {
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        let format = match args.prettify_as.as_deref() {
            Some(s) => {
                let parsed = crate::prettify::parse_format(s)?;
                if parsed == crate::prettify::Format::Unknown {
                    crate::prettify::detect(content_type, &body_str)
                } else {
                    parsed
                }
            }
            None => crate::prettify::detect(content_type, &body_str),
        };
        // When the user explicitly forced a format, propagate parse errors.
        // When auto-detect picked the format, fall back to the raw body on
        // parse failure (legacy behaviour).
        let out_text = if args.prettify_as.is_some() {
            crate::prettify::run(&body_str, format)?
        } else {
            crate::prettify::run(&body_str, format).unwrap_or(body_str)
        };
        out_text.into_bytes()
    } else {
        body_bytes
    };

    // 2.5 Optional --render: HTML → text. HTML-only; other types pass through.
    let processed_bytes: Vec<u8> = if args.render {
        if crate::render::is_html(content_type) {
            // Decode to UTF-8 using recon's source-charset detection so
            // ISO-8859/etc. pages render correctly (the original iconv case).
            let src_label = resolve_source_charset(args, content_type, &processed_bytes);
            let src = crate::text_encoding::resolve(&src_label).unwrap_or(encoding_rs::UTF_8);
            let (decoded, _, _) = src.decode(&processed_bytes);
            let opts = crate::render::RenderOpts {
                width: args.width,
                color: args.render_color,
                no_links: args.render_no_links,
            };
            crate::render::render_html(&decoded, &opts)?.into_bytes()
        } else {
            if !args.silent {
                eprintln!(
                    "! --render: response is '{}', not HTML — passing body through unchanged",
                    content_type
                );
            }
            processed_bytes
        }
    } else {
        processed_bytes
    };

    // 3. Dispatch to sink
    let byte_count = processed_bytes.len() as u64;
    match sink {
        BodySink::Writer(w) => {
            w.write_all(&processed_bytes)?;
        }
        BodySink::File(path) => {
            if args.create_dirs {
                ensure_parent_dir(path)?;
            }
            let mut file = File::create(path)?;
            file.write_all(&processed_bytes)?;
            if !args.silent {
                eprintln!("Saved to {}", path.display());
            }
        }
        BodySink::Editor => {
            // Mirror body to stdout first when -vv is active, so any error
            // spawning the editor doesn't lose the user's view of the body.
            if args.verbose >= 2 {
                let mut stdout = std::io::stdout();
                stdout.write_all(&processed_bytes)?;
            }
            let ext = pick_editor_extension(args, content_type);
            let path = crate::editor::create_temp_file(ext, &processed_bytes)
                .context("failed to write editor temp file")?;
            let flag_value = args.editor.as_deref().unwrap_or("");
            let (cfg_default, user_aliases) = crate::editor::load_editor_config();
            let resolved = crate::editor::resolve_editor(
                flag_value,
                cfg_default.as_deref(),
                &user_aliases,
            )
            .map_err(|_| anyhow::anyhow!(
                "--editor: no value given and no [editor] default in ~/.recon/config.toml"
            ))?;
            crate::editor::spawn_editor(&resolved, &path)
                .with_context(|| format!("failed to launch editor for {}", path.display()))?;
            if args.verbose >= 1 {
                eprintln!("* editor temp file: {}", path.display());
            }
        }
        BodySink::Clipboard => {
            let text = std::str::from_utf8(&processed_bytes)
                .context("clipboard write requires UTF-8 text")?;
            crate::clipboard::write_text(text)?;
            if !args.silent {
                eprintln!("Copied to clipboard ({} bytes)", processed_bytes.len());
            }
        }
    }
    Ok(byte_count)
}

/// Pick a temp-file extension for `--editor` based on the prettify format
/// (when known) or content-type sniffing.
fn pick_editor_extension(args: &crate::cli::Args, content_type: &str) -> &'static str {
    if let Some(s) = args.prettify_as.as_deref() {
        match s.to_ascii_lowercase().as_str() {
            "json" => return "json",
            "xml" => return "xml",
            "html" => return "html",
            "yaml" | "yml" => return "yaml",
            "csv" => return "csv",
            "tsv" => return "tsv",
            _ => {}
        }
    }
    crate::editor::extension_for_content_type(content_type)
}

/// `mkdir -p` for the parent of `path`, if it has one.
///
/// `Path::new("file.txt").parent()` is `Some("")`, not `None` — the empty-OsStr
/// check skips mkdir in that case (current working directory, already exists).
fn ensure_parent_dir(path: &std::path::Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create-dirs failed for {}", parent.display()))?;
        }
    }
    Ok(())
}

/// Derive a filename from the URL's last path segment. Non-validating fallback
/// used by the `-O` branch of `resolve_output_path` (unit-test path — real runs
/// go through `util::filename_from_url` in main.rs, which percent-decodes and
/// rejects path-escape sequences). Returns `"index.html"` if the URL path is
/// empty or fails to parse.
fn basename_from_url(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();
    let path = parsed.as_ref().map(|u| u.path()).unwrap_or("/");
    let last = path.rsplit('/').next().unwrap_or("").to_string();
    if last.is_empty() {
        "index.html".to_string()
    } else {
        last
    }
}

/// Pick the source charset for a response body. Priority:
///   1. `--source-charset` explicit override.
///   2. `charset=` parameter on the Content-Type header.
///   3. BOM / chardetng sniff.
fn resolve_source_charset(args: &Args, content_type: &str, bytes: &[u8]) -> String {
    if let Some(c) = &args.source_charset {
        return c.clone();
    }
    if let Some(c) = crate::text_encoding::parse_content_type_charset(content_type) {
        return c;
    }
    crate::text_encoding::detect(bytes).charset.to_string()
}

/// Parse an HTTP-date (RFC 7231 §7.1.1.1): IMF-fixdate, RFC 850, asctime.
/// Returns Unix timestamp (seconds since epoch) or None.
fn parse_http_date(s: &str) -> Option<i64> {
    httpdate::parse_http_date(s)
        .ok()
        .and_then(|sys| sys.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

#[cfg(test)]
mod path_tests {
    use super::*;

    fn args_with(output: Option<&str>, output_dir: Option<&str>, remote_name: bool) -> Args {
        let mut a = Args::test_default();
        a.output = output.map(PathBuf::from);
        a.output_dir = output_dir.map(PathBuf::from);
        a.remote_name = remote_name;
        a
    }

    #[test]
    fn output_only_uses_path_as_is() {
        let a = args_with(Some("file.txt"), None, false);
        let p = resolve_output_path(&a, "https://example.com/stuff/page.html", None).unwrap();
        assert_eq!(p, Some(PathBuf::from("file.txt")));
    }

    #[test]
    fn output_dir_prefixes_output() {
        let a = args_with(Some("file.txt"), Some("./dl"), false);
        let p = resolve_output_path(&a, "https://example.com/page.html", None).unwrap();
        assert_eq!(p, Some(PathBuf::from("./dl/file.txt")));
    }

    #[test]
    fn remote_name_derives_basename_from_url() {
        let a = args_with(None, None, true);
        let p = resolve_output_path(&a, "https://example.com/downloads/archive.tar.gz", None).unwrap();
        assert_eq!(p, Some(PathBuf::from("archive.tar.gz")));
    }

    #[test]
    fn remote_name_with_output_dir() {
        let a = args_with(None, Some("./dl"), true);
        let p = resolve_output_path(&a, "https://example.com/archive.tar.gz", None).unwrap();
        assert_eq!(p, Some(PathBuf::from("./dl/archive.tar.gz")));
    }

    #[test]
    fn remote_name_empty_url_path_defaults_to_index() {
        let a = args_with(None, None, true);
        let p = resolve_output_path(&a, "https://example.com/", None).unwrap();
        assert_eq!(p, Some(PathBuf::from("index.html")));
    }
}
