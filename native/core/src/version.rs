//! The app's version as every head shows it.

/// The package version and, when built from git, the commit:
/// `0.1.0 (2391c1d)`. Set by `build.rs`.
pub const VERSION: &str = env!("JITTER_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_the_package_version() {
        assert!(VERSION.starts_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn a_commit_is_seven_hex_characters_in_parentheses() {
        if let Some(rest) = VERSION.strip_prefix(concat!(env!("CARGO_PKG_VERSION"), " (")) {
            let commit = rest.strip_suffix(')').unwrap_or_default();
            assert_eq!(commit.len(), 7, "{VERSION}");
            assert!(commit.chars().all(|c| c.is_ascii_hexdigit()), "{VERSION}");
        } else {
            assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        }
    }
}
