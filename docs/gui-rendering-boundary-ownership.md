# Final GUI rendering ownership register

This register covers the production module graph rooted at `inputforge-gui-dx/src/lib.rs`,
with test-only items excluded, plus every browser script under the GUI assets directory.
The final audit and its verification/review record are authoritative for completion.
Module enumeration, symbol/caller inspection, a current-source scan and connected-code
review are complementary evidence; a text scan alone does not prove architecture.

## Permitted rendering mechanics

- Native event observations are forwarded to supplied app callbacks. Reading the event's
  key/coordinates without branching only serializes the observation. App callbacks own
  keyboard interpretation, propagation, validation, draft/open/error transitions,
  capture coordination, notification/readout deadlines and listener/gesture lifetime.
- Equality/prop synchronization and publication of an app-selected value or completion
  result remain renderer signal mechanics. Mounted tab registries store DOM references;
  the app chooses availability and focus targets. Native focus/selection and checkbox
  indeterminate-property writes execute rendering instructions and install no listener.
- Menu bounding-box measurement, portal relocation/cleanup and viewport placement remain
  drawing mechanics. The stable outer portal anchor is retained. Dioxus-scoped async
  measurement/focus is not an engine scheduler or workflow timer.
- Curve/deadzone SVG paths, anchors, thumbnails, scale conversion, plot hit geometry,
  labels, summary copy and CSS animation are rendering. In particular, `curve.evaluate`
  only positions a dot on the displayed draft curve using an already app-prepared scalar;
  it neither walks/evaluates a live action chain nor determines engine output.
- Stage hints/hold eligibility and optimistic keyboard-combo reconciliation are app-owned.
  Existing stage field handlers construct typed raw edit intents; app validators decide
  execution/no-op, and GUI code publishes their returned values/errors. Header/rail keys,
  menu completion and add-pad coordination likewise use existing app controllers.

## Module inventory

Each source below is loaded through the production module graph. Test harnesses and
cfg(test) fixture adapters are excluded. The private `test_support.rs` engine fixture
is compiled only for unit tests and is inaccessible through `render_editor` or AppContext.
No production fallback, compatibility engine context, sender or mutable state input remains.

| Source | Remaining responsibility |
| --- | --- |
| [src/app.rs](../crates/inputforge-gui-dx/src/app.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/batch_mapping.rs](../crates/inputforge-gui-dx/src/batch_mapping.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/bridge.rs](../crates/inputforge-gui-dx/src/bridge.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/capture.rs](../crates/inputforge-gui-dx/src/capture.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/components/badge.rs](../crates/inputforge-gui-dx/src/components/badge.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/button.rs](../crates/inputforge-gui-dx/src/components/button.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/card.rs](../crates/inputforge-gui-dx/src/components/card.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/checkbox.rs](../crates/inputforge-gui-dx/src/components/checkbox.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/chip.rs](../crates/inputforge-gui-dx/src/components/chip.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/click_away_listener.rs](../crates/inputforge-gui-dx/src/components/click_away_listener.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/dialog.rs](../crates/inputforge-gui-dx/src/components/dialog.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/drawer.rs](../crates/inputforge-gui-dx/src/components/drawer.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/field.rs](../crates/inputforge-gui-dx/src/components/field.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/icon.rs](../crates/inputforge-gui-dx/src/components/icon.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/icon_button.rs](../crates/inputforge-gui-dx/src/components/icon_button.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/integer_input.rs](../crates/inputforge-gui-dx/src/components/integer_input.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/label.rs](../crates/inputforge-gui-dx/src/components/label.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/layout.rs](../crates/inputforge-gui-dx/src/components/layout.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/menu/anchored.rs](../crates/inputforge-gui-dx/src/components/menu/anchored.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/menu/items.rs](../crates/inputforge-gui-dx/src/components/menu/items.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/menu/mod.rs](../crates/inputforge-gui-dx/src/components/menu/mod.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/menu/placement.rs](../crates/inputforge-gui-dx/src/components/menu/placement.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/menu/root.rs](../crates/inputforge-gui-dx/src/components/menu/root.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/mod.rs](../crates/inputforge-gui-dx/src/components/mod.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/number_input.rs](../crates/inputforge-gui-dx/src/components/number_input.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/portal.rs](../crates/inputforge-gui-dx/src/components/portal.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/segmented_control.rs](../crates/inputforge-gui-dx/src/components/segmented_control.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/select.rs](../crates/inputforge-gui-dx/src/components/select.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/separator.rs](../crates/inputforge-gui-dx/src/components/separator.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/slider.rs](../crates/inputforge-gui-dx/src/components/slider.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/sortable/gap.rs](../crates/inputforge-gui-dx/src/components/sortable/gap.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/sortable/handle.rs](../crates/inputforge-gui-dx/src/components/sortable/handle.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/sortable/live_region.rs](../crates/inputforge-gui-dx/src/components/sortable/live_region.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/sortable/mod.rs](../crates/inputforge-gui-dx/src/components/sortable/mod.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/sortable/state.rs](../crates/inputforge-gui-dx/src/components/sortable/state.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/spinner.rs](../crates/inputforge-gui-dx/src/components/spinner.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/status_bar.rs](../crates/inputforge-gui-dx/src/components/status_bar.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/switch.rs](../crates/inputforge-gui-dx/src/components/switch.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/tab_button.rs](../crates/inputforge-gui-dx/src/components/tab_button.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/tabs.rs](../crates/inputforge-gui-dx/src/components/tabs.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/tabs_list.rs](../crates/inputforge-gui-dx/src/components/tabs_list.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/tabs_root.rs](../crates/inputforge-gui-dx/src/components/tabs_root.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/text_input.rs](../crates/inputforge-gui-dx/src/components/text_input.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/components/tooltip.rs](../crates/inputforge-gui-dx/src/components/tooltip.rs) | Markup, mounted references, placement and supplied interaction callbacks |
| [src/context/mod.rs](../crates/inputforge-gui-dx/src/context/mod.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/controllers/inspector.rs](../crates/inputforge-gui-dx/src/controllers/inspector.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/controllers/mod.rs](../crates/inputforge-gui-dx/src/controllers/mod.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/controls.rs](../crates/inputforge-gui-dx/src/controls.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/frame/bulk_map/empty_state.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/bulk_map/labels.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/labels.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/bulk_map/metadata.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/metadata.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/bulk_map/mod.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/bulk_map/row_readout.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/bulk_map/rows.rs](../crates/inputforge-gui-dx/src/frame/bulk_map/rows.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/layout/empty_state.rs](../crates/inputforge-gui-dx/src/frame/layout/empty_state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/layout/mod.rs](../crates/inputforge-gui-dx/src/frame/layout/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/content.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/content.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/empty_state.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/empty_state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/engine_offline_banner.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/engine_offline_banner.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/header.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/inactive_hint.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/inactive_hint.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/keyboard.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/keyboard.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/in_block.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/out_block.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/out_chain.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_chain.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/predicate.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/predicate.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/live_readout/value_helpers.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/mapping_issue_copy.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/mapping_issue_copy.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/mapping_issues.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/mapping_issues.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/add_palette.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_actions_menu.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/branches.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/branches.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/change_mode.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/conditional.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/conditional.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/deadzone/rendering.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/rendering.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/deadzone/state.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/deadzone/toolbar.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/toolbar.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/gesture_body.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/gesture_body.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/bridge.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/bridge.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/geometry.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/geometry.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/live_axis.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/live_axis.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/instruments/wiring.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/wiring.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/invert.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/invert.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/map_to_mouse.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_mouse.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/predicate.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/press_gesture.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/press_gesture.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/response_curve/thumbnail.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/thumbnail.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_body/tap_gesture.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/tap_gesture.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/pipeline/stage_header.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/undo_log.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_editor/undo_recap.rs](../crates/inputforge-gui-dx/src/frame/mapping_editor/undo_recap.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/add_inline.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/add_inline_rendering.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/add_inline_rendering.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/empty.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/filter.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/group.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/group.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/mod.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/rename_inline.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/row.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/row.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mapping_list/source_label.rs](../crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/mod.rs](../crates/inputforge-gui-dx/src/frame/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/axis_settings.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/axis_settings.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/controller_session.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_session.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/device_panel/formatting.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/formatting.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/device_panel/inspector.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/inspector.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/device_panel/mod.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/mod.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/routing_controls.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/routing_controls.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/panel_slot/virtual_config.rs](../crates/inputforge-gui-dx/src/frame/panel_slot/virtual_config.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/delete_dialog.rs](../crates/inputforge-gui-dx/src/frame/profiles/delete_dialog.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/library.rs](../crates/inputforge-gui-dx/src/frame/profiles/library.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/mod.rs](../crates/inputforge-gui-dx/src/frame/profiles/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/new_profile_submode.rs](../crates/inputforge-gui-dx/src/frame/profiles/new_profile_submode.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/open_choice_submode.rs](../crates/inputforge-gui-dx/src/frame/profiles/open_choice_submode.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/row.rs](../crates/inputforge-gui-dx/src/frame/profiles/row.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/snapshot_confirm.rs](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_confirm.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/profiles/snapshot_drawer.rs](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/field_row.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/field_row.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/gestures_section.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/gestures_section.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/mod.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/prune_confirm.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/prune_confirm.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/section.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/section.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/snapshots_section.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/snapshots_section.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/settings_panel/startup_section.rs](../crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/engine_pill/logic.rs](../crates/inputforge-gui-dx/src/frame/top_bar/engine_pill/logic.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/engine_pill/mod.rs](../crates/inputforge-gui-dx/src/frame/top_bar/engine_pill/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mod.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mode_tabs/add_inline.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mode_tabs/context_menu.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mode_tabs/delete_dialog.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/delete_dialog.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mode_tabs/mod.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/mode_tabs/rename_inline.rs](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/rename_inline.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/primary_nav.rs](../crates/inputforge-gui-dx/src/frame/top_bar/primary_nav.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/profile_name.rs](../crates/inputforge-gui-dx/src/frame/top_bar/profile_name.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/tools_cluster/logic.rs](../crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/top_bar/tools_cluster/mod.rs](../crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/frame/view_state.rs](../crates/inputforge-gui-dx/src/frame/view_state.rs) | Prepared frame markup, labels/geometry and callback/result publication |
| [src/icons/mod.rs](../crates/inputforge-gui-dx/src/icons/mod.rs) | Static visual assets, formatting and theme wrappers |
| [src/instruments.rs](../crates/inputforge-gui-dx/src/instruments.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/keyboard_capture.rs](../crates/inputforge-gui-dx/src/keyboard_capture.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/lib.rs](../crates/inputforge-gui-dx/src/lib.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/mapping_edits.rs](../crates/inputforge-gui-dx/src/mapping_edits.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/mapping_hints.rs](../crates/inputforge-gui-dx/src/mapping_hints.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/mapping_rail.rs](../crates/inputforge-gui-dx/src/mapping_rail.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/mapping_ui.rs](../crates/inputforge-gui-dx/src/mapping_ui.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/mode_management.rs](../crates/inputforge-gui-dx/src/mode_management.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/mode_management_ui.rs](../crates/inputforge-gui-dx/src/mode_management_ui.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/navigation.rs](../crates/inputforge-gui-dx/src/navigation.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/overlays.rs](../crates/inputforge-gui-dx/src/overlays.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/patterns/destructive_confirm.rs](../crates/inputforge-gui-dx/src/patterns/destructive_confirm.rs) | Presentation signal wiring, composition and callback lifecycle |
| [src/patterns/dirty_confirm.rs](../crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs) | Presentation signal wiring, composition and callback lifecycle |
| [src/patterns/keyboard_capture/mod.rs](../crates/inputforge-gui-dx/src/patterns/keyboard_capture/mod.rs) | Presentation signal wiring, composition and callback lifecycle |
| [src/patterns/live_capture/mod.rs](../crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs) | Presentation signal wiring, composition and callback lifecycle |
| [src/patterns/mod.rs](../crates/inputforge-gui-dx/src/patterns/mod.rs) | Presentation signal wiring, composition and callback lifecycle |
| [src/profiles/mod.rs](../crates/inputforge-gui-dx/src/profiles/mod.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/profiles/snapshots.rs](../crates/inputforge-gui-dx/src/profiles/snapshots.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/readouts/labels.rs](../crates/inputforge-gui-dx/src/readouts/labels.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/readouts/mod.rs](../crates/inputforge-gui-dx/src/readouts/mod.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/readouts/model.rs](../crates/inputforge-gui-dx/src/readouts/model.rs) | Root contract installation, prepared signal allocation or equality-gated publication |
| [src/routing.rs](../crates/inputforge-gui-dx/src/routing.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/settings.rs](../crates/inputforge-gui-dx/src/settings.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/snapshots/config.rs](../crates/inputforge-gui-dx/src/snapshots/config.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/snapshots/live.rs](../crates/inputforge-gui-dx/src/snapshots/live.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/snapshots/meta.rs](../crates/inputforge-gui-dx/src/snapshots/meta.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/snapshots/mod.rs](../crates/inputforge-gui-dx/src/snapshots/mod.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/snapshots/settings.rs](../crates/inputforge-gui-dx/src/snapshots/settings.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/sortable.rs](../crates/inputforge-gui-dx/src/sortable.rs) | Prepared value, signal and callback contract; no engine or service access |
| [src/theme/mod.rs](../crates/inputforge-gui-dx/src/theme/mod.rs) | Static visual assets, formatting and theme wrappers |
| [src/toast/mod.rs](../crates/inputforge-gui-dx/src/toast/mod.rs) | Prepared notification markup and callback forwarding |
| [src/toast/queue.rs](../crates/inputforge-gui-dx/src/toast/queue.rs) | Prepared notification markup and callback forwarding |
| [src/toast/state.rs](../crates/inputforge-gui-dx/src/toast/state.rs) | Prepared notification markup and callback forwarding |
| [src/toast/viewport.rs](../crates/inputforge-gui-dx/src/toast/viewport.rs) | Prepared notification markup and callback forwarding |

## Browser and historical source inventory

The only `.js` path remaining under GUI assets is
[the former sortable location](../crates/inputforge-gui-dx/assets/components/sortable_drag.js).
It is comments-only, is never loaded, and points to
[the app-owned transport](../crates/inputforge-app/assets/sortable_drag.js).
[The former interaction source](../crates/inputforge-gui-dx/src/components/sortable/interaction.rs)
is likewise an unloaded comments-only historical location notice. These two files preserve
byte-unchanged historical evidence links; they are not dormant runtime implementations.
No production GUI inline script installs event listeners, deadlines, timers or animation loops.

App browser transports retain their original event/geometry algorithms and lifecycle
coverage: keyboard capture, rail/editor keyboard, sortable pointer/ghost, instrument
pointer, snapshot shortcut, navigation and overlay suites. Sortable drawing/scroll animation
is app-owned transport, not a second engine poller. Core processing and native services
remain authoritative in their existing core/app layers.

## Acceptance limits

This register establishes source ownership and automated regression coverage. It does not
claim native Wayland/Windows, physical-device/compositor, assistive-technology, visual,
latency or performance acceptance. Follow the final audit's manual checklist and retained
historical acceptance reports. Existing transitive platform support is distinct from a
direct GUI/app X11 dependency; no direct X11 API, command or workaround was introduced.

The 2026-10-18 native acceptance follow-up keeps these ownership assignments:
shared browser text acknowledgement/reset policy lives in
[`inputforge-app/src/ui/text.rs`](../crates/inputforge-app/src/ui/text.rs), while
GUI inputs render browser-owned initial values and forward events. Empty-workspace
profile entry and native picker lifecycle remain in
[`inputforge-app/src/profiles/entry.rs`](../crates/inputforge-app/src/profiles/entry.rs)
and its dialog controller. Eager batch CSS remains theme rendering, and instrument
history refresh observes existing application publications without a new poller.
