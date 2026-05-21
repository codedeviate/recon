//! Multi-line input detection. After Enter, we speculatively compile
//! the buffer; an "unexpected EOF" style error means the user is in
//! the middle of typing a multi-line construct (open brace, open
//! string, open paren), and we should show a continuation prompt.

use rhai::{Engine, ParseErrorType};

#[derive(Debug, PartialEq)]
pub enum Status {
    /// Buffer parses cleanly — ready to eval.
    Complete,
    /// Buffer is mid-statement; ask for more input.
    NeedMore,
    /// Real syntax error; report and reset.
    Syntax(String),
}

pub fn classify(engine: &Engine, src: &str) -> Status {
    if src.trim().is_empty() {
        return Status::Complete;
    }
    match engine.compile(src) {
        Ok(_) => Status::Complete,
        Err(e) => classify_error(e),
    }
}

fn classify_error(e: rhai::ParseError) -> Status {
    match e.err_type() {
        // Script ends prematurely (open brace, open block, etc.)
        ParseErrorType::UnexpectedEOF => Status::NeedMore,
        // Missing closing token: "}", ")", "]" — user is still typing
        ParseErrorType::MissingToken(tok, _) => {
            if tok == "}" || tok == ")" || tok == "]" {
                Status::NeedMore
            } else {
                Status::Syntax(e.to_string())
            }
        }
        // Unterminated string literal wraps as BadInput(LexError::UnterminatedString)
        ParseErrorType::BadInput(rhai::LexError::UnterminatedString) => Status::NeedMore,
        _ => Status::Syntax(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        Engine::new()
    }

    #[test]
    fn empty_is_complete() {
        assert_eq!(classify(&engine(), ""), Status::Complete);
        assert_eq!(classify(&engine(), "   \n  "), Status::Complete);
    }

    #[test]
    fn complete_expression() {
        assert_eq!(classify(&engine(), "1 + 2"), Status::Complete);
    }

    #[test]
    fn complete_statement() {
        assert_eq!(classify(&engine(), "let x = 5;"), Status::Complete);
    }

    #[test]
    fn open_brace_needs_more() {
        assert_eq!(classify(&engine(), "if true {"), Status::NeedMore);
    }

    #[test]
    fn open_function_needs_more() {
        assert_eq!(classify(&engine(), "fn greet(n) {"), Status::NeedMore);
    }

    #[test]
    fn open_paren_needs_more() {
        assert_eq!(classify(&engine(), "print("), Status::NeedMore);
    }

    #[test]
    fn open_string_needs_more() {
        assert_eq!(classify(&engine(), "let x = \"hello"), Status::NeedMore);
    }

    #[test]
    fn real_syntax_error() {
        match classify(&engine(), "let = 5") {
            Status::Syntax(_) => {}
            other => panic!("expected Syntax, got {other:?}"),
        }
    }
}
