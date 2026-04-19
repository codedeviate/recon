//! Liechtenstein VAT. Uses the Swiss UID system verbatim.
//!
//! Liechtenstein companies register under the Swiss UID scheme (CHE prefix,
//! same weights and check-digit algorithm). The only difference from `ch-vat`
//! is the `detected` label returned in a Valid verdict.
//!
//! Source: Swiss UID system covers FL (Fürstentum Liechtenstein) companies.
//! python-stdnum li.uid delegates to ch.uid.

use super::super::Verdict;
use anyhow::Result;

pub fn verify_li_vat(input: &str) -> Verdict {
    match super::ch::verify_ch_vat(input) {
        Verdict::Valid { formatted, comment, .. } => Verdict::Valid {
            formatted,
            detected: "Liechtenstein UID".into(),
            comment,
        },
        other => other,
    }
}

pub fn create_li_vat(input: &str, raw: bool) -> Result<String> {
    super::ch::create_ch_vat(input, raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same valid UID as CH but with Liechtenstein label.
    #[test]
    fn li_vat_delegates_to_ch_and_relabels() {
        match verify_li_vat("CHE-100.155.212") {
            Verdict::Valid { detected, formatted, .. } => {
                assert_eq!(detected, "Liechtenstein UID");
                assert_eq!(formatted, "CHE-100.155.212");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn li_vat_propagates_invalid() {
        match verify_li_vat("100155213") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn li_vat_round_trip() {
        let full = create_li_vat("10015521", false).unwrap();
        assert_eq!(full, "CHE-100.155.212");
        match verify_li_vat(&full) {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Liechtenstein UID"),
            v => panic!("{:?}", v),
        }
    }
}
