use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Response;
use std::fs::File;
use std::io::{self, Write};

use crate::cli::Args;

pub fn write_response(mut response: Response, args: &Args) -> Result<()> {
    let status = response.status();

    if args.status_only {
        println!("{}", status.as_u16());
        return Ok(());
    }

    let print_headers = args.verbose >= 1 || args.include_headers || args.head_only || args.lhead || args.full;
    let print_body = !args.head_only || args.full;

    if print_headers {
        let dest: &mut dyn Write = if args.include_headers || args.head_only || args.lhead || args.full {
            &mut io::stdout()
        } else {
            &mut io::stderr()
        };

        if args.lhead {
            writeln!(dest, "* {}", response.url())?;
        }

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

        writeln!(dest, "< {colored_status}")?;

        for (name, value) in response.headers() {
            writeln!(dest, "< {}: {}", name, value.to_str().unwrap_or("?"))?;
        }
        writeln!(dest, "<")?;
    }

    if args.fail_on_error && status.as_u16() >= 400 {
        return Err(anyhow!(
            "HTTP error {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }

    if !print_body {
        return Ok(());
    }

    if args.prettify {
        let content_type_str = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = response.text().context("Failed to read response body")?;
        let format = crate::prettify::detect(&content_type_str, &body);
        let out = crate::prettify::run(&body, format).unwrap_or_else(|_| body);
        if let Some(path) = &args.output {
            let mut file = File::create(path)?;
            write!(file, "{out}")?;
            if !args.silent {
                eprintln!("Saved to {}", path.display());
            }
        } else {
            print!("{out}");
        }
        return Ok(());
    }

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
        io::copy(&mut response, &mut io::stdout())?;
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
