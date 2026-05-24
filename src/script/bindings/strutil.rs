//! PHP-style string helpers exposed as top-level Rhai functions:
//! `trim` / `ltrim` / `rtrim`, `strrev`, `strip_html`, `nl2br`, `br2nl`,
//! `preg_match`, `preg_replace`, `printf`, `sprintf`, `urlencode` /
//! `urldecode`, `base64_encode` / `base64_decode`, `html_entity_decode`,
//! `str_pad` / `lpad` / `rpad`, `dirname` / `basename`, `date_format`.
//!
//! Free-function rather than method or namespaced shape because the
//! requested names are recognisable PHP idioms — `trim($s)` reads
//! naturally as `trim(s)`, and scripts ported from PHP can stay
//! readable. Rhai already exposes `.trim()` as a String method; this
//! module deliberately co-exists without removing it.

use crate::script::convert::err;
use base64::Engine as _;
use chrono::{DateTime, Local, TimeZone, Utc};
use regex::Regex;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult};

pub fn register(engine: &mut Engine) {
    // ---- trim family --------------------------------------------------

    // trim(s) — whitespace.
    engine.register_fn("trim", |s: &str| -> String { s.trim().to_string() });
    // trim(s, mask) — strip any char in `mask` from both ends.
    engine.register_fn("trim", |s: &str, mask: &str| -> String {
        trim_with(s, mask, true, true)
    });

    engine.register_fn("ltrim", |s: &str| -> String {
        s.trim_start().to_string()
    });
    engine.register_fn("ltrim", |s: &str, mask: &str| -> String {
        trim_with(s, mask, true, false)
    });

    engine.register_fn("rtrim", |s: &str| -> String {
        s.trim_end().to_string()
    });
    engine.register_fn("rtrim", |s: &str, mask: &str| -> String {
        trim_with(s, mask, false, true)
    });

    // ---- strrev -------------------------------------------------------

    // Unicode-aware: reverses by grapheme-less chars, which is good
    // enough for the common case (accented letters and emoji stay
    // intact). True grapheme-cluster reversal would need
    // `unicode-segmentation`; not worth the extra dep here.
    engine.register_fn("strrev", |s: &str| -> String {
        s.chars().rev().collect()
    });

    // ---- array join ---------------------------------------------------

    // join(arr, sep) — also reachable as `arr.join(sep)` since Rhai
    // dispatches method syntax through the same registration. Rhai
    // 1.24's BasicArrayPackage doesn't ship this, and recon's existing
    // `join(&mut ThreadHandle)` is the only registration users see —
    // which makes `arr.join(", ")` fail with a confusing overload-mismatch.
    // Non-string elements are coerced via Dynamic::to_string so mixed
    // arrays (numbers + strings) work the way scripts expect.
    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|d| crate::script::convert::to_string(d))
            .collect::<Vec<_>>()
            .join(sep)
    });

    // ---- HTML helpers -------------------------------------------------

    engine.register_fn("strip_html", |s: &str| -> String { strip_html(s) });
    engine.register_fn("nl2br", |s: &str| -> String { nl2br(s) });
    engine.register_fn("br2nl", |s: &str| -> String { br2nl(s) });

    // ---- regex --------------------------------------------------------

    // preg_match(pattern, subject) -> Array of captures (group 0 first,
    // then each capture group). Empty array if no match. PHP-style
    // delimiters (e.g. "/foo/i") are accepted for ergonomics — see
    // `compile_php_regex`.
    engine.register_fn(
        "preg_match",
        |pattern: &str, subject: &str| -> Result<Array, Box<EvalAltResult>> {
            let re = compile_php_regex(pattern)?;
            let mut out = Array::new();
            if let Some(caps) = re.captures(subject) {
                for c in caps.iter() {
                    let cell: Dynamic = match c {
                        Some(m) => m.as_str().to_string().into(),
                        None => "".to_string().into(),
                    };
                    out.push(cell);
                }
            }
            Ok(out)
        },
    );

    // preg_replace(pattern, replacement, subject) -> String. Replaces
    // every match. `$1` / `$2` (or `${name}`) in `replacement` expand
    // to capture groups, matching the `regex` crate's default
    // semantics.
    engine.register_fn(
        "preg_replace",
        |pattern: &str, replacement: &str, subject: &str|
         -> Result<String, Box<EvalAltResult>> {
            let re = compile_php_regex(pattern)?;
            Ok(re.replace_all(subject, replacement).into_owned())
        },
    );

    // ---- printf / sprintf --------------------------------------------

    // Three arities each: no-arg (literal format), single arg, array.
    // Variadic isn't a Rhai concept; scripts pass `[a, b, c]` for
    // multi-arg formats.
    engine.register_fn(
        "sprintf",
        |fmt: &str| -> Result<String, Box<EvalAltResult>> {
            sprintf_apply(fmt, &[])
        },
    );
    engine.register_fn(
        "sprintf",
        |fmt: &str, arg: Dynamic| -> Result<String, Box<EvalAltResult>> {
            sprintf_apply(fmt, std::slice::from_ref(&arg))
        },
    );
    engine.register_fn(
        "sprintf",
        |fmt: &str, args: Array| -> Result<String, Box<EvalAltResult>> {
            sprintf_apply(fmt, &args)
        },
    );

    engine.register_fn(
        "printf",
        |fmt: &str| -> Result<i64, Box<EvalAltResult>> {
            let s = sprintf_apply(fmt, &[])?;
            print!("{s}");
            Ok(s.len() as i64)
        },
    );
    engine.register_fn(
        "printf",
        |fmt: &str, arg: Dynamic| -> Result<i64, Box<EvalAltResult>> {
            let s = sprintf_apply(fmt, std::slice::from_ref(&arg))?;
            print!("{s}");
            Ok(s.len() as i64)
        },
    );
    engine.register_fn(
        "printf",
        |fmt: &str, args: Array| -> Result<i64, Box<EvalAltResult>> {
            let s = sprintf_apply(fmt, &args)?;
            print!("{s}");
            Ok(s.len() as i64)
        },
    );

    // ---- URL encoding ------------------------------------------------

    // urlencode(s) — RFC 3986 percent-encoding for an arbitrary string.
    // Matches the encoding browsers use for `application/x-www-form-
    // urlencoded` values plus a couple of safe ASCII chars; the
    // `urlencoding` crate uses the same allow-list as Rust's
    // `serde_urlencoded`.
    engine.register_fn("urlencode", |s: &str| -> String {
        urlencoding::encode(s).into_owned()
    });
    engine.register_fn(
        "urldecode",
        |s: &str| -> Result<String, Box<EvalAltResult>> {
            urlencoding::decode(s)
                .map(|c| c.into_owned())
                .map_err(|e| err(format!("urldecode: {e}")))
        },
    );

    // ---- base64 ------------------------------------------------------

    // base64_encode accepts either a string (encoded as UTF-8 bytes) or
    // a Blob (raw bytes). Output is always standard base64 with `=`
    // padding; the URL-safe variant lives separately so the default
    // matches PHP and most APIs.
    engine.register_fn("base64_encode", |s: &str| -> String {
        base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
    });
    engine.register_fn("base64_encode", |b: Blob| -> String {
        base64::engine::general_purpose::STANDARD.encode(&b[..])
    });

    // base64_decode returns a Blob — base64-decoded bytes aren't
    // guaranteed to be valid UTF-8, so callers convert with
    // `text::decode(b, "utf-8")` (or similar) when they want a String.
    engine.register_fn(
        "base64_decode",
        |s: &str| -> Result<Blob, Box<EvalAltResult>> {
            base64::engine::general_purpose::STANDARD
                .decode(s.trim())
                .map_err(|e| err(format!("base64_decode: {e}")))
        },
    );

    // ---- HTML entity decode ------------------------------------------

    // html_entity_decode(s) — the inverse of HTML entity encoding.
    // `strip_html` deliberately leaves entities alone (matching PHP's
    // strip_tags) so this is the natural follow-up call when scraping
    // textual content out of HTML.
    engine.register_fn("html_entity_decode", |s: &str| -> String {
        html_escape::decode_html_entities(s).into_owned()
    });

    // ---- str_pad / lpad / rpad ---------------------------------------

    // str_pad(s, width)            — pad with spaces on the right (PHP default).
    // str_pad(s, width, pad)       — custom pad string, right side.
    // str_pad(s, width, pad, side) — side is "left", "right", or "both".
    engine.register_fn("str_pad", |s: &str, width: i64| -> String {
        pad_str(s, width as usize, " ", PadSide::Right)
    });
    engine.register_fn(
        "str_pad",
        |s: &str, width: i64, pad: &str| -> String {
            pad_str(s, width as usize, pad, PadSide::Right)
        },
    );
    engine.register_fn(
        "str_pad",
        |s: &str, width: i64, pad: &str, side: &str|
         -> Result<String, Box<EvalAltResult>> {
            let side = match side.to_ascii_lowercase().as_str() {
                "left" | "l" | "start" => PadSide::Left,
                "right" | "r" | "end" => PadSide::Right,
                "both" | "center" | "centre" => PadSide::Both,
                other => return Err(err(format!(
                    "str_pad: unknown side '{other}' (want left / right / both)"
                ))),
            };
            Ok(pad_str(s, width as usize, pad, side))
        },
    );
    // lpad / rpad are the bare-name aliases people reach for from
    // SQL / Oracle / Python contexts.
    engine.register_fn("lpad", |s: &str, width: i64| -> String {
        pad_str(s, width as usize, " ", PadSide::Left)
    });
    engine.register_fn("lpad", |s: &str, width: i64, pad: &str| -> String {
        pad_str(s, width as usize, pad, PadSide::Left)
    });
    engine.register_fn("rpad", |s: &str, width: i64| -> String {
        pad_str(s, width as usize, " ", PadSide::Right)
    });
    engine.register_fn("rpad", |s: &str, width: i64, pad: &str| -> String {
        pad_str(s, width as usize, pad, PadSide::Right)
    });

    // ---- path helpers ------------------------------------------------

    // dirname / basename — POSIX semantics, applied to the literal
    // string. Path separators are `/` only (matches PHP and the cross-
    // platform behaviour scripts usually want — Windows callers should
    // pre-normalise with `replace`).
    engine.register_fn("dirname", |path: &str| -> String { dirname(path) });
    engine.register_fn("basename", |path: &str| -> String {
        basename(path, "")
    });
    engine.register_fn(
        "basename",
        |path: &str, suffix: &str| -> String { basename(path, suffix) },
    );

    // ---- date_format -------------------------------------------------

    // date_format(unix_ts, fmt)             — UTC.
    // date_format(unix_ts, fmt, "local")    — local timezone.
    // Format spec is chrono's strftime — see chrono's docs.
    engine.register_fn(
        "date_format",
        |unix_ts: i64, fmt: &str| -> Result<String, Box<EvalAltResult>> {
            format_date(unix_ts, fmt, false)
        },
    );
    engine.register_fn(
        "date_format",
        |unix_ts: i64, fmt: &str, tz: &str|
         -> Result<String, Box<EvalAltResult>> {
            let local = matches!(tz.to_ascii_lowercase().as_str(),
                "local" | "localtime" | "system");
            format_date(unix_ts, fmt, local)
        },
    );
}

// ---------------------------------------------------------------------
// trim implementation
// ---------------------------------------------------------------------

fn trim_with(s: &str, mask: &str, left: bool, right: bool) -> String {
    let chars: std::collections::HashSet<char> = mask.chars().collect();
    let mut start = 0usize;
    let mut end = s.len();
    if left {
        for (i, c) in s.char_indices() {
            if !chars.contains(&c) {
                start = i;
                break;
            }
            start = i + c.len_utf8();
        }
    }
    if right {
        let mut new_end = end;
        for (i, c) in s.char_indices().rev() {
            if !chars.contains(&c) {
                new_end = i + c.len_utf8();
                break;
            }
            new_end = i;
        }
        end = new_end;
    }
    if start >= end {
        String::new()
    } else {
        s[start..end].to_string()
    }
}

// ---------------------------------------------------------------------
// HTML helpers
// ---------------------------------------------------------------------

/// Remove every `<...>` segment. Does NOT decode HTML entities — that
/// matches PHP's `strip_tags` (entities pass through untouched). Quoted
/// attribute values are skipped so `<a title="oh >no<">` is removed
/// cleanly.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'<' {
            // Skip until the matching '>', respecting quoted attributes.
            let mut j = i + 1;
            let mut quote: Option<u8> = None;
            while j < bytes.len() {
                let c = bytes[j];
                match quote {
                    Some(q) if c == q => quote = None,
                    Some(_) => {}
                    None => match c {
                        b'"' | b'\'' => quote = Some(c),
                        b'>' => {
                            j += 1;
                            break;
                        }
                        _ => {}
                    },
                }
                j += 1;
            }
            i = j;
        } else {
            // Copy one UTF-8 codepoint at a time to keep slicing valid.
            let ch_len = utf8_char_len(b);
            out.push_str(&s[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte < 0x80 {
        1
    } else if first_byte < 0xC0 {
        // Continuation byte — shouldn't be at codepoint boundary, but
        // copy a single byte to keep progressing.
        1
    } else if first_byte < 0xE0 {
        2
    } else if first_byte < 0xF0 {
        3
    } else {
        4
    }
}

/// PHP-compatible `nl2br`: inserts `<br>` BEFORE each newline. Handles
/// `\r\n`, `\n`, and standalone `\r`. HTML5 void-element form (no
/// trailing slash) to match the rest of recon's HTML output.
fn nl2br(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if i + 1 < bytes.len() && bytes[i + 1] == b'\n' => {
                out.push_str("<br>\r\n");
                i += 2;
            }
            b'\n' => {
                out.push_str("<br>\n");
                i += 1;
            }
            b'\r' => {
                out.push_str("<br>\r");
                i += 1;
            }
            _ => {
                let n = utf8_char_len(bytes[i]);
                out.push_str(&s[i..i + n]);
                i += n;
            }
        }
    }
    out
}

/// Inverse-ish of `nl2br`. Any `<br>`, `<br/>`, `<br />` (any case,
/// any inner whitespace) becomes a single `\n`. A `<br>` immediately
/// followed by `\r\n` / `\n` / `\r` consumes that newline too, so
/// `nl2br` followed by `br2nl` round-trips cleanly.
fn br2nl(s: &str) -> String {
    // Pattern stays local to keep this stateless; the cost is one
    // re-compile per call, which is irrelevant for REPL/script use.
    // If the tag is immediately followed by an EOL, keep that EOL
    // (so `nl2br` → `br2nl` round-trips and preserves \r\n vs \n).
    // Otherwise emit a `\n`.
    let re = Regex::new(r"(?i)<br\s*/?>(\r\n|\n|\r)?").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        match caps.get(1) {
            Some(m) => m.as_str().to_string(),
            None => "\n".to_string(),
        }
    })
    .into_owned()
}

// ---------------------------------------------------------------------
// regex
// ---------------------------------------------------------------------

/// Accept either a raw regex (`r"foo\d+"`) or PHP-style delimited form
/// (`"/foo\d+/i"`). Recognised modifiers: `i` (case-insensitive),
/// `m` (multi-line), `s` (dot matches newline), `x` (verbose).
fn compile_php_regex(pattern: &str) -> Result<Regex, Box<EvalAltResult>> {
    let trimmed = pattern;
    let final_pattern: String = if let Some(stripped) = strip_php_delimiters(trimmed) {
        stripped
    } else {
        trimmed.to_string()
    };
    Regex::new(&final_pattern).map_err(|e| err(format!("preg: invalid regex: {e}")))
}

fn strip_php_delimiters(pattern: &str) -> Option<String> {
    let bytes = pattern.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let opener = bytes[0];
    // PHP allows several delimiter chars. Stick to the popular ones —
    // a script that uses `#` or `~` can switch to the raw form.
    let closer = match opener {
        b'/' => b'/',
        b'#' => b'#',
        b'~' => b'~',
        b'|' => b'|',
        _ => return None,
    };
    // Locate the last unescaped delimiter, so flags-after-closer work.
    let mut close_idx = None;
    let mut prev_backslash = false;
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if prev_backslash {
            prev_backslash = false;
            continue;
        }
        if b == b'\\' {
            prev_backslash = true;
            continue;
        }
        if b == closer {
            close_idx = Some(i);
        }
    }
    let close = close_idx?;
    if close == 0 {
        return None;
    }
    let body = &pattern[1..close];
    let flags = &pattern[close + 1..];
    let mut prefix = String::from("(?");
    let mut any_flag = false;
    for ch in flags.chars() {
        match ch {
            'i' | 'm' | 's' | 'x' => {
                prefix.push(ch);
                any_flag = true;
            }
            _ => {
                // Unknown flag — bail and let the caller try raw.
                return None;
            }
        }
    }
    if any_flag {
        prefix.push(')');
        Some(format!("{prefix}{body}"))
    } else {
        Some(body.to_string())
    }
}

// ---------------------------------------------------------------------
// printf / sprintf
// ---------------------------------------------------------------------

/// Minimal printf-style formatter. Supports `%[flags][width][.precision]spec`
/// with `spec` ∈ `d i u o x X b f e E g G s c %`. Flags supported:
/// `-` (left-align), `0` (zero-pad), `+` (force sign), ` ` (space-sign),
/// `#` (alt form for `o`/`x`/`X`/`b`).
fn sprintf_apply(fmt: &str, args: &[Dynamic]) -> Result<String, Box<EvalAltResult>> {
    let mut out = String::with_capacity(fmt.len());
    let mut idx = 0usize; // next arg
    let bytes = fmt.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            let n = utf8_char_len(bytes[i]);
            out.push_str(&fmt[i..i + n]);
            i += n;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            return Err(err("sprintf: trailing '%' with no specifier"));
        }
        // Parse flags.
        let mut left_align = false;
        let mut zero_pad = false;
        let mut force_sign = false;
        let mut space_sign = false;
        let mut alt_form = false;
        loop {
            match bytes[i] {
                b'-' => left_align = true,
                b'0' => zero_pad = true,
                b'+' => force_sign = true,
                b' ' => space_sign = true,
                b'#' => alt_form = true,
                _ => break,
            }
            i += 1;
            if i >= bytes.len() {
                return Err(err("sprintf: format ends inside flags"));
            }
        }
        // Width.
        let mut width: usize = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            width = width * 10 + (bytes[i] - b'0') as usize;
            i += 1;
        }
        // Precision.
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let mut p = 0usize;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                p = p * 10 + (bytes[i] - b'0') as usize;
                i += 1;
            }
            precision = Some(p);
        }
        if i >= bytes.len() {
            return Err(err("sprintf: format ends before specifier"));
        }
        let spec = bytes[i] as char;
        i += 1;

        if spec == '%' {
            out.push('%');
            continue;
        }

        let arg = args.get(idx).cloned().unwrap_or(Dynamic::UNIT);
        if spec != '%' {
            idx += 1;
        }

        let rendered = render_spec(
            spec,
            &arg,
            FmtFlags {
                left_align,
                zero_pad,
                force_sign,
                space_sign,
                alt_form,
                width,
                precision,
            },
        )?;
        out.push_str(&rendered);
    }
    Ok(out)
}

struct FmtFlags {
    left_align: bool,
    zero_pad: bool,
    force_sign: bool,
    space_sign: bool,
    alt_form: bool,
    width: usize,
    precision: Option<usize>,
}

fn render_spec(spec: char, arg: &Dynamic, f: FmtFlags) -> Result<String, Box<EvalAltResult>> {
    let body = match spec {
        's' => {
            let mut s = coerce_string(arg);
            if let Some(p) = f.precision {
                if s.chars().count() > p {
                    s = s.chars().take(p).collect();
                }
            }
            s
        }
        'c' => {
            // Character: int → codepoint, string → first char.
            if let Some(n) = coerce_int(arg) {
                match char::from_u32(n as u32) {
                    Some(c) => c.to_string(),
                    None => {
                        return Err(err(format!(
                            "sprintf: %c got invalid codepoint {n}"
                        )))
                    }
                }
            } else {
                coerce_string(arg).chars().next().map(|c| c.to_string()).unwrap_or_default()
            }
        }
        'd' | 'i' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err(format!("sprintf: %{spec} needs an integer")))?;
            format_int(n, 10, false, &f)
        }
        'u' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err("sprintf: %u needs an integer"))?;
            // PHP treats negatives as their two's-complement bit pattern;
            // matching that exactly is rarely useful. Just clamp to 0.
            let n = if n < 0 { 0 } else { n };
            format_int(n, 10, false, &f)
        }
        'o' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err("sprintf: %o needs an integer"))?;
            let mut s = format_int(n, 8, false, &f);
            if f.alt_form && !s.starts_with('0') {
                s.insert(0, '0');
            }
            s
        }
        'x' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err("sprintf: %x needs an integer"))?;
            let mut s = format_int(n, 16, false, &f);
            if f.alt_form {
                s.insert_str(0, "0x");
            }
            s
        }
        'X' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err("sprintf: %X needs an integer"))?;
            let mut s = format_int(n, 16, true, &f);
            if f.alt_form {
                s.insert_str(0, "0X");
            }
            s
        }
        'b' => {
            let n = coerce_int(arg)
                .ok_or_else(|| err("sprintf: %b needs an integer"))?;
            format_int(n, 2, false, &f)
        }
        'f' | 'F' => {
            let v = coerce_float(arg)
                .ok_or_else(|| err(format!("sprintf: %{spec} needs a number")))?;
            let prec = f.precision.unwrap_or(6);
            format_float(v, prec, &f, /* exp */ None)
        }
        'e' | 'E' => {
            let v = coerce_float(arg)
                .ok_or_else(|| err(format!("sprintf: %{spec} needs a number")))?;
            let prec = f.precision.unwrap_or(6);
            format_float(v, prec, &f, Some(spec == 'E'))
        }
        'g' | 'G' => {
            // Pick the shorter of %e and %f, like C / PHP.
            let v = coerce_float(arg)
                .ok_or_else(|| err(format!("sprintf: %{spec} needs a number")))?;
            let prec = f.precision.unwrap_or(6).max(1);
            let exp_form = format_float(v, prec - 1, &f, Some(spec == 'G'));
            let fix_form = format_float(v, prec, &f, None);
            if exp_form.len() < fix_form.len() {
                exp_form
            } else {
                fix_form
            }
        }
        other => {
            return Err(err(format!("sprintf: unknown specifier '%{other}'")));
        }
    };
    Ok(pad(body, &f))
}

fn coerce_string(v: &Dynamic) -> String {
    if v.is::<()>() {
        String::new()
    } else if v.is_string() {
        v.clone().into_string().unwrap_or_default()
    } else {
        v.to_string()
    }
}

fn coerce_int(v: &Dynamic) -> Option<i64> {
    if let Some(n) = v.clone().try_cast::<i64>() {
        return Some(n);
    }
    if let Some(f) = v.clone().try_cast::<f64>() {
        return Some(f as i64);
    }
    if v.is_string() {
        v.clone().into_string().ok().and_then(|s| s.trim().parse().ok())
    } else {
        None
    }
}

fn coerce_float(v: &Dynamic) -> Option<f64> {
    if let Some(f) = v.clone().try_cast::<f64>() {
        return Some(f);
    }
    if let Some(n) = v.clone().try_cast::<i64>() {
        return Some(n as f64);
    }
    if v.is_string() {
        v.clone().into_string().ok().and_then(|s| s.trim().parse().ok())
    } else {
        None
    }
}

fn format_int(n: i64, base: u32, upper: bool, f: &FmtFlags) -> String {
    let negative = n < 0;
    // For base != 10 use the bit pattern, matching C/PHP.
    let mag_str = if base == 10 {
        n.unsigned_abs().to_string()
    } else {
        let bits = n as u64;
        match base {
            2 => format!("{bits:b}"),
            8 => format!("{bits:o}"),
            16 if upper => format!("{bits:X}"),
            16 => format!("{bits:x}"),
            _ => bits.to_string(),
        }
    };
    let sign = if negative {
        "-"
    } else if f.force_sign {
        "+"
    } else if f.space_sign {
        " "
    } else {
        ""
    };
    let body = match f.precision {
        Some(p) if p > mag_str.len() => {
            let pad = "0".repeat(p - mag_str.len());
            format!("{pad}{mag_str}")
        }
        _ => mag_str,
    };
    if f.zero_pad
        && f.precision.is_none()
        && !f.left_align
        && f.width > sign.len() + body.len()
    {
        let pad = "0".repeat(f.width - sign.len() - body.len());
        format!("{sign}{pad}{body}")
    } else {
        format!("{sign}{body}")
    }
}

fn format_float(v: f64, prec: usize, f: &FmtFlags, exp: Option<bool>) -> String {
    let sign = if v.is_sign_negative() {
        "-"
    } else if f.force_sign {
        "+"
    } else if f.space_sign {
        " "
    } else {
        ""
    };
    let mag = v.abs();
    let body = match exp {
        Some(upper) => {
            let raw = format!("{:.*e}", prec, mag);
            // Rust's `e` produces `1.23e5`; PHP/C produce `1.23e+05`.
            normalise_exp(&raw, upper)
        }
        None => format!("{:.*}", prec, mag),
    };
    if f.zero_pad && !f.left_align && f.width > sign.len() + body.len() {
        let pad = "0".repeat(f.width - sign.len() - body.len());
        format!("{sign}{pad}{body}")
    } else {
        format!("{sign}{body}")
    }
}

fn normalise_exp(raw: &str, upper: bool) -> String {
    // `1.23e5` → `1.23e+05`; `1.23e-5` → `1.23e-05`.
    let e = match raw.find('e') {
        Some(i) => i,
        None => return raw.to_string(),
    };
    let (mantissa, exp_part) = raw.split_at(e);
    let exp_part = &exp_part[1..]; // skip 'e'
    let (esign, mag) = if let Some(rest) = exp_part.strip_prefix('-') {
        ('-', rest)
    } else if let Some(rest) = exp_part.strip_prefix('+') {
        ('+', rest)
    } else {
        ('+', exp_part)
    };
    let mag = if mag.len() < 2 { format!("0{mag}") } else { mag.to_string() };
    let letter = if upper { 'E' } else { 'e' };
    format!("{mantissa}{letter}{esign}{mag}")
}

fn pad(body: String, f: &FmtFlags) -> String {
    if f.width <= body.chars().count() {
        return body;
    }
    let extra = f.width - body.chars().count();
    let space = " ".repeat(extra);
    if f.left_align {
        format!("{body}{space}")
    } else {
        format!("{space}{body}")
    }
}

// ---------------------------------------------------------------------
// str_pad / lpad / rpad
// ---------------------------------------------------------------------

#[derive(Copy, Clone)]
enum PadSide {
    Left,
    Right,
    Both,
}

fn pad_str(s: &str, width: usize, pad: &str, side: PadSide) -> String {
    let s_len = s.chars().count();
    if width <= s_len || pad.is_empty() {
        return s.to_string();
    }
    let needed = width - s_len;
    match side {
        PadSide::Right => format!("{s}{}", repeat_to_width(pad, needed)),
        PadSide::Left => format!("{}{s}", repeat_to_width(pad, needed)),
        PadSide::Both => {
            // PHP gives the extra char to the right when needed is odd.
            let right = needed / 2 + needed % 2;
            let left = needed - right;
            format!(
                "{}{s}{}",
                repeat_to_width(pad, left),
                repeat_to_width(pad, right)
            )
        }
    }
}

fn repeat_to_width(pad: &str, width: usize) -> String {
    if pad.is_empty() || width == 0 {
        return String::new();
    }
    let pad_chars: Vec<char> = pad.chars().collect();
    let pad_len = pad_chars.len();
    let mut out = String::with_capacity(width);
    let mut emitted = 0usize;
    while emitted < width {
        out.push(pad_chars[emitted % pad_len]);
        emitted += 1;
    }
    out
}

// ---------------------------------------------------------------------
// path helpers
// ---------------------------------------------------------------------

/// POSIX-style dirname. Strips trailing slashes, then returns everything
/// before the last `/`. Returns `"."` for a bare filename, `"/"` for a
/// rooted single-component path, `""` for empty input.
fn dirname(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    // Strip trailing slashes (but keep the lone root slash).
    let trimmed = path.trim_end_matches('/');
    let trimmed = if trimmed.is_empty() { "/" } else { trimmed };
    match trimmed.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
        None => ".".to_string(),
    }
}

/// POSIX-style basename. Strips trailing slashes, returns everything
/// after the last `/`, optionally trims a matching suffix.
fn basename(path: &str, suffix: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { String::new() } else { "/".to_string() };
    }
    let name = match trimmed.rfind('/') {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    };
    if !suffix.is_empty() && name.ends_with(suffix) && name.len() > suffix.len() {
        name[..name.len() - suffix.len()].to_string()
    } else {
        name.to_string()
    }
}

// ---------------------------------------------------------------------
// date_format
// ---------------------------------------------------------------------

fn format_date(unix_ts: i64, fmt: &str, local: bool) -> Result<String, Box<EvalAltResult>> {
    let utc = match Utc.timestamp_opt(unix_ts, 0).single() {
        Some(t) => t,
        None => return Err(err(format!(
            "date_format: timestamp {unix_ts} is out of range"
        ))),
    };
    let formatted = if local {
        let local_dt: DateTime<Local> = utc.with_timezone(&Local);
        local_dt.format(fmt).to_string()
    } else {
        utc.format(fmt).to_string()
    };
    Ok(formatted)
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_mask_strips_chosen_chars() {
        assert_eq!(trim_with("--hello--", "-", true, true), "hello");
        assert_eq!(trim_with("xxhelloyy", "xy", true, true), "hello");
        assert_eq!(trim_with("--hello--", "-", true, false), "hello--");
        assert_eq!(trim_with("--hello--", "-", false, true), "--hello");
    }

    #[test]
    fn strip_html_removes_tags_keeps_text() {
        assert_eq!(strip_html("<p>hi</p>"), "hi");
        assert_eq!(
            strip_html(r#"<a href="x" title="oh >no<">click</a>"#),
            "click"
        );
        assert_eq!(strip_html("plain"), "plain");
        // Entities pass through (matches PHP's strip_tags).
        assert_eq!(strip_html("<b>&amp;</b>"), "&amp;");
    }

    #[test]
    fn nl2br_inserts_break_before_newlines() {
        assert_eq!(nl2br("a\nb"), "a<br>\nb");
        assert_eq!(nl2br("a\r\nb"), "a<br>\r\nb");
        assert_eq!(nl2br("a\rb"), "a<br>\rb");
        assert_eq!(nl2br("plain"), "plain");
    }

    #[test]
    fn br2nl_inverts_nl2br() {
        for sample in &["a\nb\nc", "alpha\r\nbeta", "x"] {
            let round = br2nl(&nl2br(sample));
            assert_eq!(&round, sample, "roundtrip failed for {sample:?}");
        }
        // Case + whitespace variants.
        assert_eq!(br2nl("a<BR>b"), "a\nb");
        assert_eq!(br2nl("a<br />b"), "a\nb");
        assert_eq!(br2nl("a<br/>b"), "a\nb");
    }

    #[test]
    fn preg_compile_handles_php_delimiters() {
        assert!(compile_php_regex("foo").is_ok());
        assert!(compile_php_regex("/foo/").is_ok());
        assert!(compile_php_regex("/foo/i").is_ok());
        assert!(compile_php_regex("#foo#").is_ok());
        // Invalid regex bubbles up.
        assert!(compile_php_regex("(").is_err());
    }

    #[test]
    fn sprintf_basics() {
        let one: Dynamic = "world".to_string().into();
        let n: Dynamic = 42i64.into();
        let p: Dynamic = std::f64::consts::PI.into();

        assert_eq!(sprintf_apply("hello %s", &[one.clone()]).unwrap(), "hello world");
        assert_eq!(sprintf_apply("%05d", &[n.clone()]).unwrap(), "00042");
        assert_eq!(sprintf_apply("%-5d|", &[n.clone()]).unwrap(), "42   |");
        assert_eq!(sprintf_apply("%+d", &[n.clone()]).unwrap(), "+42");
        assert_eq!(sprintf_apply("%.2f", &[p.clone()]).unwrap(), "3.14");
        assert_eq!(sprintf_apply("%x", &[255i64.into()]).unwrap(), "ff");
        assert_eq!(sprintf_apply("%#x", &[255i64.into()]).unwrap(), "0xff");
        assert_eq!(sprintf_apply("%b", &[5i64.into()]).unwrap(), "101");
        assert_eq!(sprintf_apply("100%%", &[]).unwrap(), "100%");
    }

    #[test]
    fn join_binding_handles_strings_and_mixed_types() {
        let engine = rhai::Engine::new();
        let mut engine = engine;
        super::register(&mut engine);

        let out = engine
            .eval::<String>(r#"["a", "b", "c"].join(", ")"#)
            .unwrap();
        assert_eq!(out, "a, b, c");

        // Free-function form.
        let out = engine
            .eval::<String>(r#"join(["x", "y"], "|")"#)
            .unwrap();
        assert_eq!(out, "x|y");

        // Mixed types coerce via Dynamic::to_string.
        let out = engine
            .eval::<String>(r#"[1, "two", 3.5].join("-")"#)
            .unwrap();
        assert_eq!(out, "1-two-3.5");
    }

    #[test]
    fn sprintf_e_format_normalises_exponent() {
        let v: Dynamic = 1234.5f64.into();
        let out = sprintf_apply("%.2e", &[v]).unwrap();
        assert_eq!(out, "1.23e+03");
    }

    #[test]
    fn pad_str_left_right_both() {
        assert_eq!(pad_str("hi", 5, " ", PadSide::Right), "hi   ");
        assert_eq!(pad_str("hi", 5, " ", PadSide::Left), "   hi");
        assert_eq!(pad_str("hi", 6, "-", PadSide::Both), "--hi--");
        // Odd-needed → extra char goes right (PHP convention).
        assert_eq!(pad_str("hi", 5, "-", PadSide::Both), "-hi--");
        // Width <= len leaves the string alone.
        assert_eq!(pad_str("hello", 3, " ", PadSide::Right), "hello");
        // Multi-char pad cycles.
        assert_eq!(pad_str("x", 5, "ab", PadSide::Right), "xabab");
    }

    #[test]
    fn dirname_basename_posix_semantics() {
        assert_eq!(dirname("/etc/hosts"), "/etc");
        assert_eq!(dirname("/etc/"), "/");
        assert_eq!(dirname("/etc"), "/");
        assert_eq!(dirname("hosts"), ".");
        assert_eq!(dirname(""), "");
        assert_eq!(dirname("a/b/c"), "a/b");

        assert_eq!(basename("/etc/hosts", ""), "hosts");
        assert_eq!(basename("/etc/hosts/", ""), "hosts");
        assert_eq!(basename("/", ""), "/");
        assert_eq!(basename("hosts", ""), "hosts");
        // Suffix stripping.
        assert_eq!(basename("/var/log/recon.log", ".log"), "recon");
        // Suffix that equals the full name is NOT stripped (PHP rule).
        assert_eq!(basename("/var/log/.log", ".log"), ".log");
    }

    #[test]
    fn date_format_utc_and_local() {
        // Unix epoch — universally agreed on.
        let s = format_date(0, "%Y-%m-%d %H:%M:%S", false).unwrap();
        assert_eq!(s, "1970-01-01 00:00:00");

        // A round number that's easy to spot-check by hand:
        // 1700000000 = 2023-11-14 22:13:20 UTC.
        let s = format_date(1700000000, "%Y-%m-%dT%H:%M:%SZ", false).unwrap();
        assert_eq!(s, "2023-11-14T22:13:20Z");

        // Local-tz path just exercises the with_timezone branch — value
        // depends on host TZ, so just assert the year survives.
        let s = format_date(1700000000, "%Y", true).unwrap();
        assert!(s.starts_with("202"));
    }
}
