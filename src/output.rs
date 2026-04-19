use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Response;
use std::fs::File;
use std::io::{self, Write};

use crate::cli::Args;
use crate::fail::FailMode;

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

pub fn write_response(response: Response, args: &Args) -> Result<()> {
    let mut sink = StdoutSink::Stdout;
    write_response_to(response, args, &mut sink)
}

/// Same as `write_response`, but the stdout-bound bytes go through `sink`.
pub fn write_response_to(
    mut response: Response,
    args: &Args,
    sink: &mut StdoutSink,
) -> Result<()> {
    let status = response.status();

    if args.status_only {
        let mut out = sink.writer();
        writeln!(out, "{}", status.as_u16())?;
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
        return Err(anyhow!(
            "HTTP error {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }

    if print_body {
        if args.prettify {
            let content_type_str = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let body = response.text().context("Failed to read response body")?;
            let format = crate::prettify::detect(&content_type_str, &body);
            let out_text = crate::prettify::run(&body, format).unwrap_or(body);
            if let Some(path) = &args.output {
                let mut file = File::create(path)?;
                write!(file, "{out_text}")?;
                if !args.silent {
                    eprintln!("Saved to {}", path.display());
                }
            } else {
                let mut out = sink.writer();
                write!(out, "{out_text}")?;
            }
        } else {
            let content_length = response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            if let Some(path) = &args.output {
                let mut file = File::create(path)?;
                if args.progress {
                    let pb = make_progress_bar(content_length);
                    copy_with_progress(&mut response, &mut file, &pb)?;
                    pb.finish_and_clear();
                } else {
                    io::copy(&mut response, &mut file)?;
                }
                if !args.silent {
                    eprintln!("Saved to {}", path.display());
                }
            } else {
                let mut out = sink.writer();
                io::copy(&mut response, &mut out)?;
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

pub(crate) fn make_progress_bar(total: Option<u64>) -> ProgressBar {
    match total {
        Some(len) => {
            let pb = ProgressBar::new(len);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{bytes}/{total_bytes} [{bar:40}] {bytes_per_sec} eta {eta}")
                    .unwrap()
                    .progress_chars("=> "),
            );
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
