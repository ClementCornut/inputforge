//! Single entry of a bulk mapping apply. Used by
//! `EngineCommand::SetMappingsBulk` and `Profile::set_mappings_bulk`.

use crate::types::{InputAddress, OutputAddress};

/// One row by mode pair the user committed in the bulk-map wizard.
///
/// `input` MUST be `InputAddress::Bound { device, input }`. The wizard
/// always knows the source device, so all entries it dispatches are
/// bound. The bulk-map pipeline silently skips `Unbound` entries; the
/// filter lives in `Profile::set_mappings_bulk` (covered by the
/// `engine_set_mappings_bulk_skips_entries_with_unbound_input` test).
#[derive(Debug, Clone, PartialEq)]
pub struct BulkMapEntry {
    pub input: InputAddress,
    pub mode: String,
    pub output: OutputAddress,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

    #[test]
    fn bulk_map_entry_clone_and_partial_eq() {
        let entry = BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        };
        assert_eq!(entry, entry.clone());
    }
}
