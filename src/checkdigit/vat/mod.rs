//! EU VAT check digits (per-country dispatch).

pub mod bg;
pub mod se;
pub mod dk;
pub mod fi;
pub mod de;
pub mod fr;
pub mod pl;
pub mod pt;
pub mod si;
pub mod sk;
pub mod ee;
pub mod hu;
pub mod at;
pub mod be;
pub mod lu;
pub mod nl;
pub mod it;
pub mod ro;
pub mod el;
pub mod cy;
pub mod mt;
pub mod hr;
pub mod ie;
pub mod lt;
pub mod es;
pub mod cz;
pub mod lv;
pub mod no;
pub mod uk;
pub mod ch;
pub mod li;
pub mod ru;
pub mod rs;
pub mod is;
pub mod ua;
pub mod tr;

pub use bg::{verify_bg_vat, create_bg_vat, verify_bg_egn, create_bg_egn, verify_bg_bulstat, create_bg_bulstat};
pub use se::{verify_se_vat, create_se_vat};
pub use dk::{verify_dk_vat, create_dk_vat};
pub use fi::{verify_fi_vat, create_fi_vat};
pub use de::{verify_de_vat, create_de_vat};
pub use fr::{verify_fr_vat, create_fr_vat};
pub use pl::{verify_pl_vat, create_pl_vat};
pub use pt::{verify_pt_vat, create_pt_vat};
pub use si::{verify_si_vat, create_si_vat};
pub use sk::{verify_sk_vat, create_sk_vat};
pub use ee::{verify_ee_vat, create_ee_vat};
pub use hu::{verify_hu_vat, create_hu_vat};
pub use at::{verify_at_vat, create_at_vat};
pub use be::{verify_be_vat, create_be_vat};
pub use lu::{verify_lu_vat, create_lu_vat};
pub use nl::{verify_nl_vat, create_nl_vat};
pub use it::{verify_it_vat, create_it_vat};
pub use ro::{verify_ro_vat, create_ro_vat};
pub use el::{verify_el_vat, create_el_vat};
pub use cy::{verify_cy_vat, create_cy_vat};
pub use mt::{verify_mt_vat, create_mt_vat};
pub use hr::{verify_hr_vat, create_hr_vat};
pub use ie::{verify_ie_vat, create_ie_vat};
pub use lt::{verify_lt_vat, create_lt_vat};
pub use es::{verify_es_vat, create_es_vat, verify_es_nif, create_es_nif, verify_es_nie, create_es_nie, verify_es_cif, create_es_cif};
pub use cz::{verify_cz_vat, create_cz_vat, verify_cz_legal, create_cz_legal, verify_cz_person, create_cz_person};
pub use lv::{verify_lv_vat, create_lv_vat, verify_lv_personal, create_lv_personal, verify_lv_business, create_lv_business};
pub use no::{verify_no_vat, create_no_vat};
pub use uk::{verify_uk_vat, create_uk_vat};
pub use ch::{verify_ch_vat, create_ch_vat};
pub use li::{verify_li_vat, create_li_vat};
pub use ru::{verify_ru_vat, create_ru_vat, verify_ru_legal, create_ru_legal, verify_ru_individual, create_ru_individual};
pub use rs::{verify_rs_vat, create_rs_vat};
pub use is::{verify_is_vat, create_is_vat};
pub use ua::{verify_ua_vat, create_ua_vat, verify_ua_legal, create_ua_legal, verify_ua_individual, create_ua_individual};
pub use tr::{verify_tr_vat, create_tr_vat};

use super::{sanitize, Verdict};

/// The 27 EU country codes + GR as a known alias for EL — used for
/// prefix-mismatch detection.
const KNOWN_PREFIXES: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "EL", "ES",
    "FI", "FR", "GR", "HR", "HU", "IE", "IT", "LT", "LU", "LV",
    "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK",
];

/// Strip an optional leading 2-letter country-code prefix. Accepts input
/// with or without the prefix. If a prefix is present, it must match
/// `expected_cc` (case-insensitive); Greek VAT treats `EL` and `GR` as
/// aliases in both directions. If a different known EU prefix appears,
/// returns Err with a clear mismatch message.
///
/// Input is sanitized (whitespace, dashes, dots stripped; uppercased).
pub fn strip_vat_prefix(input: &str, expected_cc: &str) -> Result<String, Verdict> {
    let clean = sanitize(input, true);
    if clean.len() < 2 {
        return Ok(clean);
    }
    let first_two = &clean[..2];
    if !first_two.chars().all(|c| c.is_ascii_alphabetic()) {
        return Ok(clean);
    }
    let expected = expected_cc.to_ascii_uppercase();
    if first_two == expected {
        return Ok(clean[2..].to_string());
    }
    // EL ↔ GR alias (Greek VAT is 'el-vat' but users may type 'GR' prefix)
    if (expected == "EL" && first_two == "GR") || (expected == "GR" && first_two == "EL") {
        return Ok(clean[2..].to_string());
    }
    // Known EU code but not the expected one → mismatch error.
    if KNOWN_PREFIXES.contains(&first_two) {
        return Err(Verdict::Invalid {
            reason: format!(
                "expected {} prefix (requested {}-vat), got {}",
                expected,
                expected.to_ascii_lowercase(),
                first_two
            ),
        });
    }
    // Two alphabetic chars that aren't a known EU code — pass through as body.
    Ok(clean)
}

#[cfg(test)]
mod mod_tests {
    use super::*;

    #[test]
    fn strip_prefix_accepts_bare_body() {
        let out = strip_vat_prefix("5261040828", "PL").unwrap();
        assert_eq!(out, "5261040828");
    }

    #[test]
    fn strip_prefix_strips_matching() {
        let out = strip_vat_prefix("PL5261040828", "PL").unwrap();
        assert_eq!(out, "5261040828");
    }

    #[test]
    fn strip_prefix_strips_lowercase() {
        let out = strip_vat_prefix("pl5261040828", "PL").unwrap();
        assert_eq!(out, "5261040828");
    }

    #[test]
    fn strip_prefix_rejects_mismatched_eu() {
        match strip_vat_prefix("DE5261040828", "PL") {
            Err(Verdict::Invalid { reason }) => {
                assert!(reason.contains("PL"));
                assert!(reason.contains("DE"));
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn strip_prefix_gr_accepted_for_el() {
        let out = strip_vat_prefix("GR094259216", "EL").unwrap();
        assert_eq!(out, "094259216");
    }

    #[test]
    fn strip_prefix_el_accepted_for_gr_request() {
        let out = strip_vat_prefix("EL094259216", "GR").unwrap();
        assert_eq!(out, "094259216");
    }
}
