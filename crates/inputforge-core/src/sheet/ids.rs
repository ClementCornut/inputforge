// Rust guideline compliant 2026-05-13

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Ulid::new().to_string())
            }

            #[must_use]
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

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

stable_id_type!(TemplateId);
stable_id_type!(AssetId);
stable_id_type!(SheetId);
stable_id_type!(TemplateInstanceId);
stable_id_type!(AnchorId);
stable_id_type!(BlockId);
stable_id_type!(LineId);
stable_id_type!(MappingMetadataId);
stable_id_type!(RecoverySnapshotId);

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
        assert!(toml.contains("template-stick-left"));
    }
}
