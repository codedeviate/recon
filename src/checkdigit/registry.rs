//! Static registry of all check-digit specs. Resolve by canonical name or alias.

use super::Spec;

/// All registered Specs. Populated as algorithm modules come online.
pub static SPECS: &[&Spec] = &[
    // Populated by later tasks.
];

/// Resolve a CLI keyword (canonical or alias, case-insensitive).
pub fn resolve(name: &str) -> Option<&'static Spec> {
    let lower = name.to_ascii_lowercase();
    for spec in SPECS {
        if spec.canonical.eq_ignore_ascii_case(&lower) {
            return Some(*spec);
        }
        for alias in spec.aliases {
            if alias.eq_ignore_ascii_case(&lower) {
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
    fn empty_registry_resolves_nothing() {
        assert_eq!(SPECS.len(), 0);
    }
}
