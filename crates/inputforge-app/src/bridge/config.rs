//! Prepare device configuration and the selected mapping.
use super::{
    actions::{derive_glyphs, derive_referenced_devices, first_vjoy_output},
    devices::{build_device_display_names, build_device_panel_rows},
};
use inputforge_core::state::AppState;
use inputforge_gui_dx::snapshots::{ConfigSnapshot, MappingKey, MappingSummary};
use std::collections::{HashMap, HashSet};
pub(super) fn project(s: &AppState, selection: Option<&MappingKey>) -> ConfigSnapshot {
    let mut mapped_inputs = HashSet::new();
    let mut mapping_names = HashMap::new();
    let mut mappings = Vec::new();
    let mut selected_mapping_actions: Option<Vec<inputforge_core::action::Action>> = None;
    if let Some(profile) = &s.active_profile {
        for mapping in profile.mappings() {
            mapped_inputs.insert(mapping.input.clone());
            if let Some(name) = &mapping.name {
                mapping_names.insert(mapping.input.clone(), name.clone());
            }
            mappings.push(MappingSummary {
                input: mapping.input.clone(),
                mode: mapping.mode.clone(),
                name: mapping.name.clone(),
                glyphs: derive_glyphs(&mapping.actions),
                referenced_devices: derive_referenced_devices(&mapping.input, &mapping.actions),
                first_vjoy_output: first_vjoy_output(&mapping.actions),
            });
            if let Some((sel_mode, sel_input)) = selection
                && mapping.mode == *sel_mode
                && mapping.input == *sel_input
            {
                selected_mapping_actions = Some(mapping.actions.clone());
            }
        }
    }
    ConfigSnapshot {
        controllers: s
            .active_profile
            .as_ref()
            .and_then(|p| p.controllers())
            .cloned(),
        devices: s.devices.clone(),
        virtual_devices: s.virtual_devices.clone(),
        mapped_inputs,
        mapping_names,
        mappings,
        selected_mapping_actions,
        selected_mapping_key: selection.cloned(),
        device_panel_rows: build_device_panel_rows(s),
        device_display_names: build_device_display_names(s),
    }
}
