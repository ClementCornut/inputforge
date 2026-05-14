// Rust guideline compliant 2026-05-13

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! stable_id_type {
    ($name:ident, $summary:literal) => {
        #[doc = $summary]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Generates a new ULID-backed ID.
            #[must_use]
            pub fn new() -> Self {
                Self(Ulid::new().to_string())
            }

            /// Imports an existing persisted ID string.
            ///
            /// This intentionally does not validate ULID shape because fixtures and migrations may
            /// preserve older or future stable IDs.
            #[must_use]
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the persisted string representation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

stable_id_type!(TemplateId, "Identifies a mapping sheet template.");
stable_id_type!(AssetId, "Identifies an imported mapping sheet asset.");
stable_id_type!(SheetId, "Identifies a mapping sheet.");
stable_id_type!(
    TemplateInstanceId,
    "Identifies a template instance on a mapping sheet."
);
stable_id_type!(AnchorId, "Identifies an anchor in a mapping sheet.");
stable_id_type!(BlockId, "Identifies a block in a mapping sheet.");
stable_id_type!(LineId, "Identifies a line in a mapping sheet.");
stable_id_type!(MappingMetadataId, "Identifies mapping sheet metadata.");
stable_id_type!(
    RecoverySnapshotId,
    "Identifies a mapping sheet recovery snapshot."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_non_empty_and_unique() {
        let first = SheetId::new();
        let second = SheetId::new();

        assert!(!first.as_str().is_empty());
        assert_ne!(first, second);
    }

    #[test]
    fn id_roundtrips_as_toml_string() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper {
            id: TemplateId,
        }

        let wrapper = Wrapper {
            id: TemplateId::from_string("template-stick-left"),
        };

        let toml = toml::to_string(&wrapper).unwrap();
        let roundtripped: Wrapper = toml::from_str(&toml).unwrap();

        assert_eq!(roundtripped, wrapper);
        assert_eq!(toml, "id = \"template-stick-left\"\n");
    }
}
