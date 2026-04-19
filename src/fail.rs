use crate::cli::Args;

/// How the CLI treats HTTP error responses (status >= 400).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailMode {
    /// No fail flag — always exit 0 regardless of status.
    Off,
    /// `-f` / `--fail` — exit non-zero, body is NOT written.
    OnError,
    /// `--fail-with-body` — exit non-zero, body IS still written.
    OnErrorKeepBody,
}

impl FailMode {
    /// Derive the active fail mode from parsed args. When both `-f` and
    /// `--fail-with-body` are set, `--fail-with-body` wins (curl typically
    /// documents `--fail-with-body` as a refinement over `-f`).
    pub fn from_args(args: &Args) -> FailMode {
        if args.fail_with_body {
            FailMode::OnErrorKeepBody
        } else if args.fail_on_error {
            FailMode::OnError
        } else {
            FailMode::Off
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_args(fail: bool, fail_with_body: bool) -> Args {
        let mut a = Args::test_default();
        a.fail_on_error = fail;
        a.fail_with_body = fail_with_body;
        a
    }

    #[test]
    fn off_when_neither_set() {
        assert_eq!(FailMode::from_args(&mk_args(false, false)), FailMode::Off);
    }

    #[test]
    fn on_error_when_only_f() {
        assert_eq!(FailMode::from_args(&mk_args(true, false)), FailMode::OnError);
    }

    #[test]
    fn keep_body_when_only_fail_with_body() {
        assert_eq!(
            FailMode::from_args(&mk_args(false, true)),
            FailMode::OnErrorKeepBody
        );
    }

    #[test]
    fn keep_body_wins_when_both() {
        assert_eq!(
            FailMode::from_args(&mk_args(true, true)),
            FailMode::OnErrorKeepBody
        );
    }
}
