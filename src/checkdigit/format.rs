//! Format helpers used by multiple algorithms.

/// Group characters into fixed-width chunks separated by `sep`.
/// Example: `group_fixed("4111111111111111", 4, ' ')` → `"4111 1111 1111 1111"`.
pub fn group_fixed(s: &str, width: usize, sep: char) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / width);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && i % width == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

/// Group with variable-width segments. `groups` sums to `s.len()`.
/// Example: `group_variable("378282246310005", &[4, 6, 5], ' ')` → `"3782 822463 10005"`.
pub fn group_variable(s: &str, groups: &[usize], sep: char) -> String {
    let mut out = String::with_capacity(s.len() + groups.len());
    let mut chars = s.chars();
    for (i, &n) in groups.iter().enumerate() {
        if i > 0 {
            out.push(sep);
        }
        for _ in 0..n {
            if let Some(c) = chars.next() {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_fixed_4_space() {
        assert_eq!(group_fixed("4111111111111111", 4, ' '), "4111 1111 1111 1111");
    }

    #[test]
    fn group_variable_amex() {
        assert_eq!(group_variable("378282246310005", &[4, 6, 5], ' '), "3782 822463 10005");
    }
}
