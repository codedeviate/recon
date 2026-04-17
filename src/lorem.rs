//! Deterministic local lorem ipsum generator. No HTTP, no randomness.

use crate::sampledata::{CountSpec, CountUnit};

/// A corpus of classic lorem ipsum sentences, kept as a single flat pool
/// so the generator can deterministically slice paragraphs / words /
/// characters without any per-run state.
const CORPUS: &[&str] = &[
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",
    "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
    "Curabitur pretium tincidunt lacus nulla gravida orci a odio.",
    "Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris.",
    "Integer in mauris eu nibh euismod gravida.",
    "Duis ac tellus et risus vulputate vehicula.",
    "Donec lobortis risus a elit. Etiam tempor.",
    "Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam.",
    "Maecenas fermentum consequat mi.",
    "Donec fermentum. Pellentesque malesuada nulla a mi.",
    "Duis sapien sem, aliquet nec, commodo eget, consequat quis, neque.",
    "Aliquam faucibus, elit ut dictum aliquet, felis nisl adipiscing sapien, sed malesuada diam lacus eget erat.",
    "Cras mollis scelerisque nunc.",
    "Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis, laoreet et, pretium ac, nisi.",
    "Aenean magna nisl, mollis quis, molestie eu, feugiat in, orci.",
    "In hac habitasse platea dictumst.",
    "Fusce convallis, mauris imperdiet gravida bibendum, nisl turpis suscipit mauris, sed placerat ipsum urna sed risus.",
];

/// Generate lorem ipsum text for the given `CountSpec`.
///
/// - Unit `P` (or `None` — defaults to paragraphs): emit `n` paragraphs
///   separated by a blank line.
/// - Unit `W`: emit `n` words from the corpus joined with spaces, with
///   sentence-case punctuation every 8–15 words (deterministic pattern).
/// - Unit `C`: emit a prefix of the concatenated corpus, truncated to
///   exactly `n` characters. If truncation would land mid-word, back up
///   to the previous word boundary and pad with whitespace to reach `n`.
pub fn generate(spec: CountSpec) -> String {
    match spec.unit.unwrap_or(CountUnit::P) {
        CountUnit::P => generate_paragraphs(spec.n),
        CountUnit::W => generate_words(spec.n),
        CountUnit::C => generate_characters(spec.n),
    }
}

fn generate_paragraphs(n: u32) -> String {
    let mut out = String::new();
    for i in 0..n {
        if i > 0 {
            out.push_str("\n\n");
        }
        // Each paragraph: 3 sentences chosen deterministically by rotating
        // through the corpus.
        let start = (i as usize * 3) % CORPUS.len();
        for s in 0..3 {
            if s > 0 {
                out.push(' ');
            }
            out.push_str(CORPUS[(start + s) % CORPUS.len()]);
        }
    }
    out
}

fn generate_words(n: u32) -> String {
    // Flatten the corpus into a single stream of words, stripping trailing
    // punctuation so we control punctuation ourselves.
    let pool: Vec<&str> = CORPUS
        .iter()
        .flat_map(|s| s.split_whitespace())
        .collect();
    if pool.is_empty() || n == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut since_sentence: u32 = 0;
    let sentence_breaks = [12, 9, 14, 8, 11, 15]; // deterministic rhythm
    let mut break_idx = 0usize;

    for i in 0..n {
        if i > 0 {
            out.push(' ');
        }
        let raw = pool[(i as usize) % pool.len()];
        let word: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '\'')
            .collect();
        if i == 0 || since_sentence == 0 {
            // Capitalise at sentence start.
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.push_str(&first.to_uppercase().to_string());
                out.push_str(chars.as_str());
            }
        } else {
            out.push_str(&word.to_lowercase());
        }
        since_sentence += 1;
        if since_sentence >= sentence_breaks[break_idx % sentence_breaks.len()] {
            out.push('.');
            since_sentence = 0;
            break_idx += 1;
        }
    }
    if !out.ends_with('.') {
        out.push('.');
    }
    out
}

fn generate_characters(n: u32) -> String {
    if n == 0 {
        return String::new();
    }
    let target = n as usize;
    let joined = CORPUS.join(" ");
    let source = if joined.len() >= target {
        joined
    } else {
        // Repeat the corpus until it's long enough.
        let mut s = String::with_capacity(target + joined.len());
        while s.len() < target {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&joined);
        }
        s
    };

    if source.len() == target {
        return source;
    }

    // Truncate, then back up to the previous word boundary. Pad with spaces
    // if we backed up past the target.
    let mut cut = target;
    while cut > 0 && !source.is_char_boundary(cut) {
        cut -= 1;
    }
    if cut < source.len() {
        let bytes = source.as_bytes();
        // If the char at `cut` is not a space and the char before `cut` is
        // not a space, we're mid-word. Back up to the previous space.
        if cut < bytes.len() && bytes[cut] != b' ' {
            while cut > 0 && bytes[cut - 1] != b' ' {
                cut -= 1;
            }
            // `cut` now points just past the space before the current word;
            // we want to end at a space, so back up by one more if possible.
            if cut > 0 && bytes[cut - 1] == b' ' {
                cut -= 1;
            }
        }
    }
    let mut truncated: String = source.chars().take(cut).collect();
    while truncated.chars().count() < target {
        truncated.push(' ');
    }
    // `chars().count()` is cheap here because the corpus is ASCII; still,
    // ensure byte-length target is satisfied.
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(n: u32, unit: Option<CountUnit>) -> CountSpec {
        CountSpec { n, unit }
    }

    #[test]
    fn paragraphs_default_unit_is_p() {
        let out = generate(spec(2, None));
        let paras: Vec<&str> = out.split("\n\n").collect();
        assert_eq!(paras.len(), 2);
        for p in paras {
            assert!(!p.is_empty());
            assert!(p.ends_with('.'), "paragraph should end with a period: {p}");
        }
    }

    #[test]
    fn paragraphs_explicit_unit() {
        let out = generate(spec(3, Some(CountUnit::P)));
        assert_eq!(out.split("\n\n").count(), 3);
    }

    #[test]
    fn paragraphs_zero_is_empty() {
        assert_eq!(generate(spec(0, Some(CountUnit::P))), "");
    }

    #[test]
    fn paragraphs_are_deterministic() {
        assert_eq!(
            generate(spec(3, Some(CountUnit::P))),
            generate(spec(3, Some(CountUnit::P))),
        );
    }

    #[test]
    fn words_exact_count() {
        let out = generate(spec(50, Some(CountUnit::W)));
        // Count whitespace-separated tokens; strip trailing punctuation.
        let n = out.split_whitespace().count();
        assert_eq!(n, 50, "got: {out}");
    }

    #[test]
    fn words_ends_with_period() {
        let out = generate(spec(10, Some(CountUnit::W)));
        assert!(out.ends_with('.'));
    }

    #[test]
    fn words_zero_is_empty() {
        assert_eq!(generate(spec(0, Some(CountUnit::W))), "");
    }

    #[test]
    fn characters_exact_length() {
        for target in [1u32, 10, 100, 1000] {
            let out = generate(spec(target, Some(CountUnit::C)));
            assert_eq!(
                out.chars().count(),
                target as usize,
                "target={target}, got length {} (output: {out:?})",
                out.chars().count(),
            );
        }
    }

    #[test]
    fn characters_zero_is_empty() {
        assert_eq!(generate(spec(0, Some(CountUnit::C))), "");
    }

    #[test]
    fn characters_deterministic() {
        assert_eq!(
            generate(spec(200, Some(CountUnit::C))),
            generate(spec(200, Some(CountUnit::C))),
        );
    }
}
