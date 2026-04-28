//! BLAKE3 hashing over canonical-round-tripped TOML.
//!
//! See decision D14 in the F6 design spec.

// Rust guideline compliant 2026-04-28

use crate::error::Result;

/// Hash a profile TOML body via canonical-round-trip + BLAKE3.
///
/// Round-trips `body` through `toml::Value` so the hash is stable across
/// whitespace, comment placement, and top-level key reordering. See
/// decision D14.
///
/// # Errors
///
/// Returns [`crate::error::EngineError::ProfileParse`] if `body` is not
/// valid TOML. Re-serialization (`toml::to_string`) for valid `Value`
/// trees is infallible in practice but is mapped to `ProfileWrite`
/// for completeness.
#[allow(dead_code, reason = "called by snapshot::create in Task 8")]
pub(crate) fn hash_canonical_toml(body: &str) -> Result<[u8; 32]> {
    let value: toml::Value = toml::from_str(body)?;
    let canonical = toml::to_string(&value)?;
    Ok(*blake3::hash(canonical.as_bytes()).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two TOMLs that differ only in whitespace, comments, and key order
    /// must hash to the same value (D14).
    #[test]
    fn canonical_hash_is_stable_across_reformat() {
        let a = "name = \"x\"\n\n# comment\nbar = 2\nfoo = 1\n";
        let b = "foo = 1\nbar = 2\nname = \"x\"\n";
        assert_eq!(
            hash_canonical_toml(a).unwrap(),
            hash_canonical_toml(b).unwrap()
        );
    }

    #[test]
    fn canonical_hash_differs_on_value_change() {
        let a = "foo = 1\n";
        let b = "foo = 2\n";
        assert_ne!(
            hash_canonical_toml(a).unwrap(),
            hash_canonical_toml(b).unwrap()
        );
    }

    #[test]
    fn invalid_toml_returns_err() {
        let bad = "not = valid = toml";
        hash_canonical_toml(bad).unwrap_err();
    }
}
