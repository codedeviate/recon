//! Local lorem ipsum generator. Randomized per invocation with a xorshift64
//! PRNG; optional seed makes output reproducible. Always starts with
//! "Lorem ipsum".

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

/// xorshift64 — cheap, adequate for filler text generation. Not
/// cryptographic.
fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Pick a uniformly random index in `0..len`. Caller must ensure `len > 0`.
fn pick(state: &mut u64, len: usize) -> usize {
    (next_u64(state) as usize) % len
}

/// Generate lorem ipsum text for the given `CountSpec` seeded by `seed`.
///
/// Output always begins with the words "Lorem ipsum" (truncated to the
/// available length if `n` is smaller than 11 in `C` mode, or smaller than
/// 2 in `W` mode, etc.).
///
/// - Unit `P` (or `None` — defaults to paragraphs): emit `n` paragraphs
///   separated by a blank line. The first paragraph's first sentence is
///   always the classic `CORPUS[0]` opener; every other sentence is random.
/// - Unit `W`: emit `n` words from the corpus joined with spaces. The first
///   two words are always "Lorem" and "ipsum"; remaining words are random.
///   Sentence breaks follow a fixed rhythm (every 8–15 words).
/// - Unit `C`: emit exactly `n` characters. Starts with "Lorem ipsum"
///   (truncated if `n < 11`); for larger `n`, appends random words and
///   truncates at a word boundary, padding with spaces if necessary.
///
/// A `seed` of 0 is internally rewritten to 1 because xorshift64 has a
/// fixed point at 0.
pub fn generate(spec: CountSpec, seed: u64) -> String {
    let mut state = if seed == 0 { 1 } else { seed };
    match spec.unit.unwrap_or(CountUnit::P) {
        CountUnit::P => generate_paragraphs(spec.n, &mut state),
        CountUnit::W => generate_words(spec.n, &mut state),
        CountUnit::C => generate_characters(spec.n, &mut state),
    }
}

fn generate_paragraphs(n: u32, state: &mut u64) -> String {
    let mut out = String::new();
    for i in 0..n {
        if i > 0 {
            out.push_str("\n\n");
        }
        // Each paragraph: 3 sentences. The very first sentence of the first
        // paragraph is always CORPUS[0] ("Lorem ipsum dolor sit amet, …") so
        // the output reliably opens with "Lorem ipsum". Every other sentence
        // is random.
        for s in 0..3 {
            if s > 0 {
                out.push(' ');
            }
            let sentence = if i == 0 && s == 0 {
                CORPUS[0]
            } else {
                CORPUS[pick(state, CORPUS.len())]
            };
            out.push_str(sentence);
        }
    }
    out
}

fn generate_words(n: u32, state: &mut u64) -> String {
    let pool: Vec<&str> = CORPUS
        .iter()
        .flat_map(|s| s.split_whitespace())
        .collect();
    if pool.is_empty() || n == 0 {
        return String::new();
    }
    // First two positions are always "Lorem" and "ipsum".
    const FIXED_PREFIX: &[&str] = &["Lorem", "ipsum"];

    let mut out = String::new();
    let mut since_sentence: u32 = 0;
    let sentence_breaks = [12u32, 9, 14, 8, 11, 15];
    let mut break_idx = 0usize;

    for i in 0..n {
        if i > 0 {
            out.push(' ');
        }
        let idx = i as usize;
        if idx < FIXED_PREFIX.len() {
            // Fixed opener: "Lorem" (capitalised) then "ipsum" (lowercase).
            let fw = FIXED_PREFIX[idx];
            if idx == 0 {
                out.push_str(fw);
            } else {
                out.push_str(&fw.to_lowercase());
            }
        } else {
            let raw = pool[pick(state, pool.len())];
            let word: String = raw
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '\'')
                .collect();
            if since_sentence == 0 {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    out.push_str(&first.to_uppercase().to_string());
                    out.push_str(chars.as_str());
                }
            } else {
                out.push_str(&word.to_lowercase());
            }
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

fn generate_characters(n: u32, state: &mut u64) -> String {
    if n == 0 {
        return String::new();
    }
    let target = n as usize;
    const PREFIX: &str = "Lorem ipsum";

    // If target fits inside the fixed prefix, truncate the prefix itself.
    // PREFIX is ASCII so byte index == char index.
    if target <= PREFIX.len() {
        return PREFIX[..target].to_string();
    }

    let pool: Vec<&str> = CORPUS
        .iter()
        .flat_map(|s| s.split_whitespace())
        .collect();

    // Start with the fixed prefix and append random words until we exceed
    // target length.
    let mut source = String::with_capacity(target + 32);
    source.push_str(PREFIX);
    while source.len() < target {
        source.push(' ');
        source.push_str(pool[pick(state, pool.len())]);
    }

    if source.len() == target {
        return source;
    }

    // Truncate at a byte boundary, then back up to the previous word
    // boundary — but never below the fixed prefix. Pad with spaces so char
    // count == target exactly.
    let mut cut = target;
    while cut > 0 && !source.is_char_boundary(cut) {
        cut -= 1;
    }
    if cut < source.len() {
        let bytes = source.as_bytes();
        if cut < bytes.len() && bytes[cut] != b' ' {
            while cut > PREFIX.len() && bytes[cut - 1] != b' ' {
                cut -= 1;
            }
            if cut > PREFIX.len() && bytes[cut - 1] == b' ' {
                cut -= 1;
            }
        }
    }
    if cut < PREFIX.len() {
        cut = PREFIX.len();
    }
    let mut truncated: String = source.chars().take(cut).collect();
    while truncated.chars().count() < target {
        truncated.push(' ');
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u64 = 42;

    fn spec(n: u32, unit: Option<CountUnit>) -> CountSpec {
        CountSpec { n, unit }
    }

    #[test]
    fn paragraphs_default_unit_is_p() {
        let out = generate(spec(2, None), TEST_SEED);
        let paras: Vec<&str> = out.split("\n\n").collect();
        assert_eq!(paras.len(), 2);
        for p in paras {
            assert!(!p.is_empty());
            assert!(p.ends_with('.'), "paragraph should end with a period: {p}");
        }
    }

    #[test]
    fn paragraphs_explicit_unit() {
        let out = generate(spec(3, Some(CountUnit::P)), TEST_SEED);
        assert_eq!(out.split("\n\n").count(), 3);
    }

    #[test]
    fn paragraphs_zero_is_empty() {
        assert_eq!(generate(spec(0, Some(CountUnit::P)), TEST_SEED), "");
    }

    #[test]
    fn words_exact_count() {
        let out = generate(spec(50, Some(CountUnit::W)), TEST_SEED);
        let n = out.split_whitespace().count();
        assert_eq!(n, 50, "got: {out}");
    }

    #[test]
    fn words_ends_with_period() {
        let out = generate(spec(10, Some(CountUnit::W)), TEST_SEED);
        assert!(out.ends_with('.'));
    }

    #[test]
    fn words_zero_is_empty() {
        assert_eq!(generate(spec(0, Some(CountUnit::W)), TEST_SEED), "");
    }

    #[test]
    fn characters_exact_length() {
        for target in [1u32, 10, 100, 1000] {
            let out = generate(spec(target, Some(CountUnit::C)), TEST_SEED);
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
        assert_eq!(generate(spec(0, Some(CountUnit::C)), TEST_SEED), "");
    }

    #[test]
    fn seeded_output_is_reproducible() {
        for unit in [CountUnit::P, CountUnit::W, CountUnit::C] {
            let a = generate(spec(5, Some(unit)), 12345);
            let b = generate(spec(5, Some(unit)), 12345);
            assert_eq!(a, b, "same seed should produce same output for {unit:?}");
        }
    }

    #[test]
    fn different_seeds_produce_different_output() {
        let a = generate(spec(100, Some(CountUnit::W)), 1);
        let b = generate(spec(100, Some(CountUnit::W)), 2);
        assert_ne!(a, b, "different seeds should produce different 100-word output");
    }

    #[test]
    fn seed_zero_is_equivalent_to_seed_one() {
        // xorshift64 fixed point at 0; our code rewrites 0 → 1.
        let a = generate(spec(3, Some(CountUnit::P)), 0);
        let b = generate(spec(3, Some(CountUnit::P)), 1);
        assert_eq!(a, b);
    }

    #[test]
    fn paragraphs_always_start_with_lorem_ipsum() {
        for seed in [1u64, 42, 999, 7, u64::MAX] {
            let out = generate(spec(3, Some(CountUnit::P)), seed);
            assert!(
                out.starts_with("Lorem ipsum"),
                "seed={seed} got: {out:?}"
            );
        }
    }

    #[test]
    fn words_always_start_with_lorem_ipsum() {
        for seed in [1u64, 42, 999] {
            let out = generate(spec(5, Some(CountUnit::W)), seed);
            assert!(
                out.starts_with("Lorem ipsum"),
                "seed={seed} got: {out:?}"
            );
        }
    }

    #[test]
    fn words_one_word_is_lorem() {
        let out = generate(spec(1, Some(CountUnit::W)), TEST_SEED);
        // One word + the trailing period inserted at end.
        assert_eq!(out, "Lorem.");
    }

    #[test]
    fn words_two_words_are_lorem_ipsum() {
        let out = generate(spec(2, Some(CountUnit::W)), TEST_SEED);
        // Two words + trailing period.
        assert_eq!(out, "Lorem ipsum.");
    }

    #[test]
    fn characters_always_start_with_lorem_ipsum() {
        for target in [11u32, 12, 20, 50, 200, 1000] {
            let out = generate(spec(target, Some(CountUnit::C)), TEST_SEED);
            assert!(
                out.starts_with("Lorem ipsum"),
                "target={target} got: {out:?}"
            );
        }
    }

    #[test]
    fn characters_short_target_truncates_prefix() {
        assert_eq!(generate(spec(1, Some(CountUnit::C)), TEST_SEED), "L");
        assert_eq!(generate(spec(5, Some(CountUnit::C)), TEST_SEED), "Lorem");
        assert_eq!(generate(spec(6, Some(CountUnit::C)), TEST_SEED), "Lorem ");
        assert_eq!(generate(spec(10, Some(CountUnit::C)), TEST_SEED), "Lorem ipsu");
        assert_eq!(generate(spec(11, Some(CountUnit::C)), TEST_SEED), "Lorem ipsum");
    }
}
