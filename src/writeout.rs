use anyhow::{Context, Result};
use std::io::Read;

/// A single token in a parsed format string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Literal(String),
    Variable(String),       // %{name}
    Header(String),         // %{header{name}}
    Json,                   // %{json}
    StderrSwitch,           // %{stderr}
    StdoutSwitch,           // %{stdout}
}

/// Load a format string argument: plain string, `@<file>`, or `@-` (stdin).
pub fn load_format(arg: &str) -> Result<String> {
    if arg == "@-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).context("reading stdin for -w")?;
        return Ok(s);
    }
    if let Some(path) = arg.strip_prefix('@') {
        return std::fs::read_to_string(path)
            .with_context(|| format!("reading format file: {path}"));
    }
    Ok(arg.to_string())
}

/// Parse a format string into tokens.
/// Unknown %{var} tokens are preserved as `Token::Variable(name)` so the
/// renderer (Task 12) can decide whether to emit literally or error.
pub fn parse(format: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut lit = String::new();
    let mut iter = format.char_indices().peekable();

    while let Some((i, c)) = iter.next() {
        // Escape sequences: \n \t \r \\, unknown preserved as "\X"
        if c == '\\' {
            if let Some(&(_, next)) = iter.peek() {
                match next {
                    'n' => { lit.push('\n'); iter.next(); continue; }
                    't' => { lit.push('\t'); iter.next(); continue; }
                    'r' => { lit.push('\r'); iter.next(); continue; }
                    '\\' => { lit.push('\\'); iter.next(); continue; }
                    other => {
                        lit.push('\\');
                        lit.push(other);
                        iter.next();
                        continue;
                    }
                }
            }
        }

        // %% → literal %
        if c == '%' {
            if let Some(&(_, '%')) = iter.peek() {
                lit.push('%');
                iter.next();
                continue;
            }
            // %{...}
            if let Some(&(_, '{')) = iter.peek() {
                // Byte-based matching-brace search starting at `i` in the full string
                if let Some(end_rel) = find_matching_brace(&format.as_bytes()[i..]) {
                    // end_rel is index of matching '}' relative to i; body is format[i+2..i+end_rel]
                    let inner = &format[i + 2..i + end_rel];
                    if !lit.is_empty() {
                        tokens.push(Token::Literal(std::mem::take(&mut lit)));
                    }
                    tokens.push(classify_token(inner));
                    // Advance iter past the closing brace
                    // We need to skip chars until we reach byte index i + end_rel + 1
                    let target = i + end_rel + 1;
                    while let Some(&(j, _)) = iter.peek() {
                        if j >= target { break; }
                        iter.next();
                    }
                    continue;
                }
                // Unmatched %{ — fall through and treat '%' as literal
            }
        }

        lit.push(c);
    }

    if !lit.is_empty() {
        tokens.push(Token::Literal(lit));
    }
    tokens
}

/// Given bytes starting with `%{`, return the index of the matching `}`
/// relative to the start. Handles nested braces for `%{header{name}}`.
fn find_matching_brace(s: &[u8]) -> Option<usize> {
    debug_assert_eq!(&s[..2], b"%{");
    let mut depth = 1;
    let mut i = 2;
    while i < s.len() {
        match s[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn classify_token(inner: &str) -> Token {
    if inner == "json" { return Token::Json; }
    if inner == "stderr" { return Token::StderrSwitch; }
    if inner == "stdout" { return Token::StdoutSwitch; }
    if let Some(rest) = inner.strip_prefix("header{") {
        if let Some(name) = rest.strip_suffix('}') {
            return Token::Header(name.to_string());
        }
    }
    Token::Variable(inner.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_literal() {
        assert_eq!(parse("hello"), vec![Token::Literal("hello".into())]);
    }

    #[test]
    fn escapes() {
        assert_eq!(
            parse(r"line1\nline2\t\\end"),
            vec![Token::Literal("line1\nline2\t\\end".into())]
        );
    }

    #[test]
    fn double_percent() {
        assert_eq!(parse("50%%"), vec![Token::Literal("50%".into())]);
    }

    #[test]
    fn simple_variable() {
        assert_eq!(parse("%{http_code}"), vec![Token::Variable("http_code".into())]);
    }

    #[test]
    fn variable_embedded_in_literal() {
        assert_eq!(
            parse("status=%{http_code}\n"),
            vec![
                Token::Literal("status=".into()),
                Token::Variable("http_code".into()),
                Token::Literal("\n".into()),
            ]
        );
    }

    #[test]
    fn header_extraction() {
        assert_eq!(
            parse("%{header{Content-Type}}"),
            vec![Token::Header("Content-Type".into())]
        );
    }

    #[test]
    fn json_token() {
        assert_eq!(parse("%{json}"), vec![Token::Json]);
    }

    #[test]
    fn stream_switches() {
        assert_eq!(
            parse("out%{stderr}err%{stdout}back"),
            vec![
                Token::Literal("out".into()),
                Token::StderrSwitch,
                Token::Literal("err".into()),
                Token::StdoutSwitch,
                Token::Literal("back".into()),
            ]
        );
    }

    #[test]
    fn unknown_variable_preserved() {
        assert_eq!(
            parse("%{unknown_xyz}"),
            vec![Token::Variable("unknown_xyz".into())]
        );
    }

    #[test]
    fn unterminated_brace_treated_as_literal() {
        // %{no closing brace → the '%' should emit literally; assert we don't crash
        let result = parse("%{open");
        assert!(!result.is_empty());
    }

    #[test]
    fn preserves_utf8_in_literal() {
        // Regression: byte-based parser would corrupt this to garbage
        assert_eq!(
            parse("é=%{http_code}"),
            vec![
                Token::Literal("é=".into()),
                Token::Variable("http_code".into()),
            ]
        );
    }

    #[test]
    fn load_plain_string() {
        assert_eq!(load_format("hello").unwrap(), "hello");
    }

    #[test]
    fn load_at_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "from file").unwrap();
        let arg = format!("@{}", tmp.path().display());
        assert_eq!(load_format(&arg).unwrap(), "from file");
    }
}
