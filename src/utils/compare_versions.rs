use std::cmp::Ordering;

/// Compare two semver versions (handles prerelease versions).
///
/// Semver rules:
/// - 0.3.0-beta.1 < 0.3.0 (prerelease is less than release)
/// - 0.3.0-alpha < 0.3.0-beta (alphabetically)
/// - 0.3.0-beta.1 < 0.3.0-beta.2 (numerically)
/// - 0.3.0-beta.16 < 0.3.0-rc.0 (alphabetically: "beta" < "rc")
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    // Split into base version and prerelease parts on first "-"
    let (a_base, a_prerelease) = split_version(a);
    let (b_base, b_prerelease) = split_version(b);

    // Parse base version parts
    let a_base_parts = parse_base(a_base);
    let b_base_parts = parse_base(b_base);
    let max_base_len = a_base_parts.len().max(b_base_parts.len());

    // Compare base versions first
    for i in 0..max_base_len {
        let a_val = a_base_parts.get(i).copied().unwrap_or(0);
        let b_val = b_base_parts.get(i).copied().unwrap_or(0);
        match a_val.cmp(&b_val) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }

    // Base versions are equal, compare prerelease
    // No prerelease > prerelease (1.0.0 > 1.0.0-beta)
    match (a_prerelease, b_prerelease) {
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (None, None) => return Ordering::Equal,
        (Some(a_pre), Some(b_pre)) => {
            // Both have prerelease, compare them
            let a_parts: Vec<&str> = a_pre.split('.').collect();
            let b_parts: Vec<&str> = b_pre.split('.').collect();
            let max_pre_len = a_parts.len().max(b_parts.len());

            for i in 0..max_pre_len {
                let a_p = a_parts.get(i);
                let b_p = b_parts.get(i);

                // Missing part means shorter prerelease comes first
                match (a_p, b_p) {
                    (None, _) => return Ordering::Less,
                    (_, None) => return Ordering::Greater,
                    (Some(a_part), Some(b_part)) => {
                        let a_num = a_part.parse::<u64>();
                        let b_num = b_part.parse::<u64>();
                        let a_is_num =
                            a_num.is_ok() && a_num.as_ref().unwrap().to_string() == *a_part;
                        let b_is_num =
                            b_num.is_ok() && b_num.as_ref().unwrap().to_string() == *b_part;

                        // Numeric identifiers have lower precedence than string identifiers
                        match (a_is_num, b_is_num) {
                            (true, false) => return Ordering::Less,
                            (false, true) => return Ordering::Greater,
                            (true, true) => {
                                let ord = a_num.unwrap().cmp(&b_num.unwrap());
                                if ord != Ordering::Equal {
                                    return ord;
                                }
                            }
                            (false, false) => {
                                let ord = a_part.cmp(b_part);
                                if ord != Ordering::Equal {
                                    return ord;
                                }
                            }
                        }
                    }
                }
            }

            Ordering::Equal
        }
    }
}

/// Split a version string into (base, prerelease) on the first "-".
fn split_version(v: &str) -> (&str, Option<&str>) {
    match v.find('-') {
        Some(idx) => (&v[..idx], Some(&v[idx + 1..])),
        None => (v, None),
    }
}

/// Parse a base version string like "1.2.3" into a vec of numbers.
fn parse_base(v: &str) -> Vec<u64> {
    v.split('.')
        .map(|n| n.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_versions() {
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
    }

    #[test]
    fn test_greater_major() {
        assert_eq!(compare_versions("2.0.0", "1.0.0"), Ordering::Greater);
    }

    #[test]
    fn test_less_minor() {
        assert_eq!(compare_versions("1.0.0", "1.1.0"), Ordering::Less);
    }

    #[test]
    fn test_prerelease_less_than_release() {
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn test_release_greater_than_prerelease() {
        assert_eq!(compare_versions("1.0.0", "1.0.0-beta"), Ordering::Greater);
    }

    #[test]
    fn test_alpha_less_than_beta() {
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0-beta"),
            Ordering::Less
        );
    }

    #[test]
    fn test_numeric_prerelease_comparison() {
        assert_eq!(
            compare_versions("1.0.0-beta.1", "1.0.0-beta.2"),
            Ordering::Less
        );
    }

    #[test]
    fn test_beta_less_than_rc() {
        assert_eq!(
            compare_versions("0.3.0-beta.16", "0.3.0-rc.0"),
            Ordering::Less
        );
    }

    #[test]
    fn test_shorter_prerelease_comes_first() {
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0-beta.1"),
            Ordering::Less
        );
    }

    // --- Additional tests ported from TypeScript ---

    #[test]
    fn test_equal_versions_010() {
        assert_eq!(compare_versions("0.1.0", "0.1.0"), Ordering::Equal);
    }

    #[test]
    fn test_greater_major_across_boundary() {
        assert_eq!(compare_versions("1.0.0", "0.9.9"), Ordering::Greater);
    }

    #[test]
    fn test_less_minor_010_020() {
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Ordering::Less);
    }

    #[test]
    fn test_prerelease_less_than_release_100() {
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn test_release_greater_than_prerelease_100() {
        assert_eq!(compare_versions("1.0.0", "1.0.0-beta"), Ordering::Greater);
    }

    #[test]
    fn test_alpha_less_than_beta_100() {
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0-beta"),
            Ordering::Less
        );
    }

    #[test]
    fn test_beta_less_than_rc_030() {
        assert_eq!(
            compare_versions("0.3.0-beta.16", "0.3.0-rc.0"),
            Ordering::Less
        );
    }

    #[test]
    fn test_numeric_prerelease_comparison_beta() {
        assert_eq!(
            compare_versions("1.0.0-beta.1", "1.0.0-beta.2"),
            Ordering::Less
        );
    }

    #[test]
    fn test_shorter_prerelease_first() {
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0-beta.1"),
            Ordering::Less
        );
    }

    #[test]
    fn test_beta2_less_than_beta10() {
        // Numeric comparison, not alphabetic: 2 < 10
        assert_eq!(
            compare_versions("0.3.0-beta.2", "0.3.0-beta.10"),
            Ordering::Less
        );
    }

    #[test]
    fn test_rc_stable_boundary() {
        assert_eq!(
            compare_versions("0.3.0-rc.0", "0.3.0"),
            Ordering::Less
        );
    }

    #[test]
    fn test_patch_comparison() {
        assert_eq!(compare_versions("0.1.1", "0.1.0"), Ordering::Greater);
    }
}
