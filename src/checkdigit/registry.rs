//! Static registry of all check-digit specs. Resolve by canonical name or alias.

use super::brand::Brand;
use super::{brand, luhn, Spec, Verdict};
use anyhow::Result;

static SPEC_LUHN: Spec = Spec {
    canonical: "luhn",
    aliases: &[],
    description: "Bare Luhn mod-10 check on any digit string",
    verify_fn: luhn::verify_bare,
    create_fn: luhn::create_bare,
};

static SPEC_CREDITCARD: Spec = Spec {
    canonical: "creditcard",
    aliases: &[],
    description: "Credit card (auto-detects brand from IIN)",
    verify_fn: brand::verify_creditcard,
    create_fn: brand::create_creditcard,
};

// Trampoline functions for brand-specific specs (closures don't coerce to fn pointers).
fn verify_visa(i: &str) -> Verdict { brand::verify_brand(i, Brand::Visa) }
fn create_visa(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Visa, r) }

fn verify_mastercard(i: &str) -> Verdict { brand::verify_brand(i, Brand::Mastercard) }
fn create_mastercard(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Mastercard, r) }

fn verify_amex(i: &str) -> Verdict { brand::verify_brand(i, Brand::Amex) }
fn create_amex(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Amex, r) }

fn verify_discover(i: &str) -> Verdict { brand::verify_brand(i, Brand::Discover) }
fn create_discover(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Discover, r) }

fn verify_jcb(i: &str) -> Verdict { brand::verify_brand(i, Brand::Jcb) }
fn create_jcb(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Jcb, r) }

static SPEC_VISA: Spec = Spec {
    canonical: "visa",
    aliases: &[],
    description: "Visa credit card (Luhn, 13/16/19 digits, IIN 4)",
    verify_fn: verify_visa,
    create_fn: create_visa,
};

static SPEC_MASTERCARD: Spec = Spec {
    canonical: "mastercard",
    aliases: &["mc"],
    description: "MasterCard (Luhn, 16 digits, IIN 51-55 or 2221-2720)",
    verify_fn: verify_mastercard,
    create_fn: create_mastercard,
};

static SPEC_AMEX: Spec = Spec {
    canonical: "amex",
    aliases: &[],
    description: "American Express (Luhn, 15 digits, IIN 34 or 37)",
    verify_fn: verify_amex,
    create_fn: create_amex,
};

static SPEC_DISCOVER: Spec = Spec {
    canonical: "discover",
    aliases: &[],
    description: "Discover (Luhn, 16 digits, IIN 6011/65/644-649)",
    verify_fn: verify_discover,
    create_fn: create_discover,
};

static SPEC_JCB: Spec = Spec {
    canonical: "jcb",
    aliases: &[],
    description: "JCB (Luhn, 16 digits, IIN 3528-3589)",
    verify_fn: verify_jcb,
    create_fn: create_jcb,
};

static SPEC_IMEI: Spec = Spec {
    canonical: "imei",
    aliases: &[],
    description: "Mobile IMEI (Luhn, 15 digits)",
    verify_fn: brand::verify_imei,
    create_fn: brand::create_imei,
};

pub static SPECS: &[&Spec] = &[
    &SPEC_LUHN,
    &SPEC_CREDITCARD,
    &SPEC_VISA,
    &SPEC_MASTERCARD,
    &SPEC_AMEX,
    &SPEC_DISCOVER,
    &SPEC_JCB,
    &SPEC_IMEI,
];

/// Resolve a CLI keyword (canonical or alias, case-insensitive).
pub fn resolve(name: &str) -> Option<&'static Spec> {
    for spec in SPECS {
        if spec.canonical.eq_ignore_ascii_case(name) {
            return Some(*spec);
        }
        for alias in spec.aliases {
            if alias.eq_ignore_ascii_case(name) {
                return Some(*spec);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_unknown_returns_none() {
        assert!(resolve("nonexistent").is_none());
    }

    #[test]
    fn resolve_luhn_returns_spec() {
        let spec = resolve("luhn").expect("luhn should resolve");
        assert_eq!(spec.canonical, "luhn");
    }

    #[test]
    fn resolve_is_case_insensitive() {
        assert!(resolve("LUHN").is_some());
        assert!(resolve("Luhn").is_some());
    }

    #[test]
    fn resolve_creditcard_returns_spec() {
        let spec = resolve("creditcard").expect("creditcard should resolve");
        assert_eq!(spec.canonical, "creditcard");
    }

    #[test]
    fn resolve_mc_alias_returns_mastercard() {
        let spec = resolve("mc").expect("mc alias should resolve");
        assert_eq!(spec.canonical, "mastercard");
    }

    #[test]
    fn resolve_imei_returns_spec() {
        let spec = resolve("imei").expect("imei should resolve");
        assert_eq!(spec.canonical, "imei");
    }
}
