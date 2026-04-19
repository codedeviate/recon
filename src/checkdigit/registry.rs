//! Static registry of all check-digit specs. Resolve by canonical name or alias.

use super::{luhn, Spec};

static SPEC_LUHN: Spec = Spec {
    canonical: "luhn",
    aliases: &[],
    description: "Bare Luhn mod-10 check on any digit string",
    verify_fn: luhn::verify_bare,
    create_fn: luhn::create_bare,
};

pub static SPECS: &[&Spec] = &[
    &SPEC_LUHN,
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
}
