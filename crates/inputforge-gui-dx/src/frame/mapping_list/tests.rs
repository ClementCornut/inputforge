//! Component tests for `frame::mapping_list`. Each test mounts a
//! stub-context harness (mirroring `app::tests::app_root_view_with_stub_contexts`)
//! and asserts on the rendered HTML.

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::state::AppState;

use crate::components::sortable::use_sortable_state;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, SettingsSnapshot};
use crate::frame::mapping_list::MappingList;
use crate::patterns::live_capture::use_live_capture_provider;

fn provide_minimal_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let profile = inputforge_core::profile::Profile::new(
        "Rail fixture".into(),
        vec![],
        inputforge_core::mode::Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let shared = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let prepared = inputforge_app::bridge::prepare(&shared.read(), None);
    let ctx = {
        crate::test_support::provide_engine(shared, cmd_tx);
        AppContext {
            settings: use_signal(SettingsSnapshot::default),
            meta: use_signal(|| prepared.meta),
            config: use_signal(ConfigSnapshot::default),
            live: use_signal(LiveSnapshot::default),
        }
    };
    use_context_provider(|| ctx.clone());

    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);

    inputforge_app::notifications::use_provider();

    crate::frame::mapping_editor::use_editor_state_provider();
    use_live_capture_provider();
}

#[test]
fn mapping_list_mounts_with_rail_class() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail"),
        "MappingList should render the .if-rail container; got: {html}",
    );
}

#[test]
fn mapping_list_renders_single_row_device_filter_chips() {
    use crate::context::{GlyphFlags, MappingSummary};
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg_signal = ctx.config;
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot {
                devices: vec![
                    DeviceState {
                        info: DeviceInfo {
                            id: DeviceId("stick".to_owned()),
                            name: "Twin Stick".to_owned(),
                            axes: 1,
                            buttons: 1,
                            hats: 0,
                            instance_path: None,
                            axis_polarities: vec![AxisPolarity::Bipolar],
                        },
                        connected: true,
                        diagnostics: DeviceDiagnostics::default(),
                    },
                    DeviceState {
                        info: DeviceInfo {
                            id: DeviceId("pedals".to_owned()),
                            name: "Pedals".to_owned(),
                            axes: 1,
                            buttons: 0,
                            hats: 0,
                            instance_path: None,
                            axis_polarities: vec![AxisPolarity::Bipolar],
                        },
                        connected: true,
                        diagnostics: DeviceDiagnostics::default(),
                    },
                ],
                mappings: vec![
                    MappingSummary {
                        input: InputAddress::Bound {
                            device: DeviceId("stick".to_owned()),
                            input: InputId::Axis { index: 0 },
                        },
                        mode: "Default".to_owned(),
                        name: Some("Pitch".to_owned()),
                        glyphs: GlyphFlags::default(),
                        referenced_devices: vec![DeviceId("stick".to_owned())],
                        first_vjoy_output: None,
                    },
                    MappingSummary {
                        input: InputAddress::Bound {
                            device: DeviceId("pedals".to_owned()),
                            input: InputId::Axis { index: 0 },
                        },
                        mode: "Default".to_owned(),
                        name: Some("Rudder".to_owned()),
                        glyphs: GlyphFlags::default(),
                        referenced_devices: vec![DeviceId("pedals".to_owned())],
                        first_vjoy_output: None,
                    },
                ],
                device_display_names: std::collections::HashMap::from([
                    (DeviceId("stick".to_owned()), "Twin Stick".to_owned()),
                    (DeviceId("pedals".to_owned()), "Pedals".to_owned()),
                ]),
                ..ConfigSnapshot::default()
            });
        });
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail__device-filter"),
        "chip strip missing: {html}"
    );
    assert!(
        html.contains("role=\"group\""),
        "chip group role missing: {html}"
    );
    assert!(
        html.contains("aria-label=\"Filter mappings by device\""),
        "chip group aria-label missing: {html}"
    );
    assert!(html.contains("Twin Stick"), "first chip missing: {html}");
    assert!(html.contains("Pedals"), "second chip missing: {html}");
}

#[test]
fn mapping_list_device_chips_are_toggle_buttons() {
    use crate::context::{GlyphFlags, MappingSummary};
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg_signal = ctx.config;
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot {
                devices: vec![DeviceState {
                    info: DeviceInfo {
                        id: DeviceId("stick".to_owned()),
                        name: "Twin Stick".to_owned(),
                        axes: 1,
                        buttons: 1,
                        hats: 0,
                        instance_path: None,
                        axis_polarities: vec![AxisPolarity::Bipolar],
                    },
                    connected: true,
                    diagnostics: DeviceDiagnostics::default(),
                }],
                mappings: vec![MappingSummary {
                    input: InputAddress::Bound {
                        device: DeviceId("stick".to_owned()),
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Default".to_owned(),
                    name: Some("Boost".to_owned()),
                    glyphs: GlyphFlags::default(),
                    referenced_devices: vec![DeviceId("stick".to_owned())],
                    first_vjoy_output: None,
                }],
                device_display_names: std::collections::HashMap::from([(
                    DeviceId("stick".to_owned()),
                    "Twin Stick".to_owned(),
                )]),
                ..ConfigSnapshot::default()
            });
        });
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail__device-chip"),
        "chip class missing: {html}"
    );
    assert!(
        html.contains("type=\"button\""),
        "chip type missing: {html}"
    );
    assert!(
        html.contains("aria-pressed=\"false\""),
        "chip pressed state missing: {html}"
    );
    assert!(
        html.contains("title=\"Twin Stick\""),
        "chip title missing: {html}"
    );
}

#[test]
fn mapping_list_add_inline_is_in_sticky_footer() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail__add-sticky"),
        "sticky footer missing: {html}"
    );
}

#[test]
fn row_renders_name_and_source_line() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Boost"), "name must render: {html}");
    assert!(html.contains("dev"), "source device must render: {html}");
    assert!(
        html.contains("if-row__source-input"),
        "input identity cell must render alongside the device cell so the \
         second half of the trigger stays visible per source_label::split_label's \
         docstring: {html}",
    );
    assert!(
        html.contains("Btn 1"),
        "button index must render in 1-indexed form (`Btn 1` for InputId::Button {{ index: 0 }}): {html}",
    );
    assert!(html.contains("if-row"), "row root class missing: {html}");
}

#[test]
fn row_omits_unnamed_placeholder_when_not_renaming() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: None,
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        !html.contains("(unnamed)"),
        "unnamed placeholder must be omitted: {html}"
    );
    assert!(
        html.contains("dev"),
        "source device must remain visible: {html}"
    );
    assert!(
        html.contains("if-row__source-input"),
        "input identity cell must render even when the row has no name, \
         so the trigger reads as `<device> . <input id>`: {html}",
    );
}

#[test]
fn row_renders_compact_vjoy_output_badge() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Pitch".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: Some(OutputAddress {
                device: 2,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Device 2 · X"),
        "virtual device output missing: {html}"
    );
    assert!(
        html.contains("if-chip--output"),
        "output chip class missing: {html}"
    );
    assert!(
        html.contains("if-row__source-input"),
        "input identity cell must render alongside the output chip so the \
         row reads as `<device> . <input id> -> <output>`: {html}",
    );
    assert!(
        html.contains("\u{2192}"),
        "source line must include the arrow glyph U+2192 separating trigger from output: {html}",
    );
}

#[test]
fn row_active_class_when_selected() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: true,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains(r#"data-selected="true""#),
        "active row must carry is-active class: {html}"
    );
}

#[test]
fn row_glyphs_render_for_merge_and_conditional() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("glyph-merge"),
        "merge glyph class must render: {html}"
    );
    assert!(
        html.contains("glyph-cond"),
        "conditional glyph class must render: {html}"
    );
}

/// Locks the row consumer's plumbing of the qualifier tooltip after the
/// 27c9445 fix that restored the lost-on-Chip-migration tooltip. Chip's
/// own tests cover the primitive's title-prop forwarding; this asserts
/// the row actually feeds `Merge: ...` / `Condition: ...` titles to
/// the qualifier Chips.
#[test]
fn row_qualifier_chips_render_with_tooltip_titles() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("title=\"Merge: "),
        "merge qualifier Chip must carry a `Merge: ...` tooltip title \
         (regression guard for the 27c9445 Chip-migration tooltip fix): {html}",
    );
    assert!(
        html.contains("title=\"Condition: "),
        "conditional qualifier Chip must carry a `Condition: ...` tooltip title \
         (regression guard for the 27c9445 Chip-migration tooltip fix): {html}",
    );
}

#[test]
fn rename_inline_renders_input_with_initial_value() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::rename_inline::RenameInline;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| Some(summary.input.clone()));
        rsx! {
            RenameInline { summary: summary, state: renaming }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "rename input must carry the .if-row-rename class: {html}",
    );
    assert!(
        html.contains("Boost"),
        "rename input must initialize with the existing name: {html}",
    );
}

#[test]
fn row_swaps_in_rename_inline_when_renaming_matches_input() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| Some(summary.input.clone()));
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "Row must swap in RenameInline when renaming matches the row's input: {html}",
    );
    assert!(
        !html.contains("if-row__name\""),
        "Row must NOT render the resting name div while renaming: {html}",
    );
    assert!(
        html.contains("if-row__source"),
        "Row must STILL render the source line during rename so the user keeps a handle on which row they are editing: {html}",
    );
    assert!(
        html.contains("dev"),
        "Source device text must remain visible during rename: {html}",
    );
    assert!(
        html.contains("if-row__source-input"),
        "Input identity cell must remain visible during rename so the \
         user keeps a handle on which physical input they are renaming: {html}",
    );
}

#[test]
fn row_renders_resting_when_renaming_is_none() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row__name"),
        "Row must render the resting name div when not renaming: {html}",
    );
    assert!(
        !html.contains("if-row-rename"),
        "Row must NOT render the rename input when renaming is None: {html}",
    );
}

#[test]
fn empty_zero_mappings_renders_title_and_class() {
    use crate::frame::mapping_list::empty::EmptyZeroMappings;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroMappings {}
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("No mappings yet"), "title missing: {html}");
    assert!(
        html.contains("if-rail-empty"),
        "rail-empty class missing: {html}"
    );
}

#[test]
fn empty_zero_filter_results_quotes_query() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "ailerons".to_owned(),
                device_label: None,
                on_clear_text: move |()| {},
                on_clear_device: None,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ailerons"),
        "filtered-empty title must quote the current query: {html}",
    );
    assert!(
        html.contains("Clear text"),
        "clear text button missing: {html}"
    );
}

#[test]
fn mapping_list_zero_filter_exposes_clear_actions() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "throttle".to_owned(),
                device_label: Some("Twin Stick".to_owned()),
                on_clear_text: move |()| {},
                on_clear_device: Some(EventHandler::new(move |()| {})),
            }
        }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Clear text"),
        "text clear action missing: {html}"
    );
    assert!(
        html.contains("Clear device"),
        "device clear action missing: {html}"
    );
    assert!(
        html.contains("Twin Stick"),
        "device label missing from zero-filter state: {html}"
    );
}

/// Pins the empty-state ghost buttons to `ButtonSize::Sm` so a future
/// regression to default Md (which would shoulder past the surrounding
/// helper-text rhythm) is caught at the test layer.
#[test]
fn empty_zero_filter_results_pins_buttons_to_sm() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "throttle".to_owned(),
                device_label: Some("Twin Stick".to_owned()),
                on_clear_text: move |()| {},
                on_clear_device: Some(EventHandler::new(move |()| {})),
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let sm_count = html.matches("if-button--sm").count();
    assert!(
        sm_count >= 2,
        "both clear-text and clear-device ghost buttons must carry \
         if-button--sm; found {sm_count} occurrence(s) in: {html}",
    );
    assert!(
        !html.contains("if-button--md"),
        "ghost buttons in the zero-filter empty state must not fall back to \
         the default Md size: {html}",
    );
}

#[test]
fn add_inline_resting_renders_dashed_row() {
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| false);
        let pending_duplicate: Signal<Option<crate::mapping_rail::Target>> = use_signal(|| None);
        rsx! {
            AddInline {
                force_expanded: force_expanded,
                pending_duplicate: pending_duplicate,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-add-inline"),
        "AddInline root class missing: {html}",
    );
    assert!(
        html.contains("Add mapping") || html.contains("+ "),
        "resting state must advertise the add affordance: {html}",
    );
}

#[test]
fn add_inline_force_expanded_arms_capture() {
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| true);
        let pending_duplicate: Signal<Option<crate::mapping_rail::Target>> = use_signal(|| None);
        rsx! {
            AddInline {
                force_expanded: force_expanded,
                pending_duplicate: pending_duplicate,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // SSR rebuild_in_place re-creates the ROOT scope each time, so we can't
    // observe `cap.active` flipping in a parent scope across rebuilds (each
    // rebuild gets a fresh LiveCapture context). We instead assert that the
    // AddInline child rendered the unified `--pad` modifier AND the chip's
    // `--listening` modifier, which is only emitted when the state machine
    // transitioned to `Pad { phase: Capturing }` AND the `cap.start`
    // callback was invoked from the same use_hook on first mount.
    assert!(
        html.contains("if-add-inline--pad"),
        "force_expanded=true must mount the pad shell; got: {html}",
    );
    assert!(
        html.contains("if-chip--listening"),
        "Capturing phase must render the listening chip modifier; got: {html}",
    );
    assert!(
        html.contains("Press an input"),
        "armed prompt text must render; got: {html}",
    );
}

#[test]
fn mapping_list_renders_axes_and_buttons_groups_in_order() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let mut mappings = vec![];
        for i in 0..3 {
            mappings.push(Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: i },
                },
                mode: "Default".to_owned(),
                name: Some(format!("Axis{i}")),
                actions: vec![Action::MapToVJoy {
                    output: OutputAddress {
                        device: 1,
                        output: OutputId::Axis { id: VJoyAxis::X },
                    },
                }],
            });
        }
        mappings.push(Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        });

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let mut cfg_signal = use_context::<AppContext>().config;
        let mut meta_signal = use_context::<AppContext>().meta;
        use_hook(move || {
            let cfg = inputforge_app::bridge::prepare(&state, None).config;
            cfg_signal.set(cfg);
            let meta = inputforge_app::bridge::prepare(&state, None).meta;
            meta_signal.set(meta);
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let axes_pos = html.find("AXES").expect("AXES header missing");
    let buttons_pos = html.find("BUTTONS").expect("BUTTONS header missing");
    assert!(
        axes_pos < buttons_pos,
        "AXES must render before BUTTONS; got: {html}",
    );
    assert!(html.contains("Axis0"));
    assert!(html.contains("Axis1"));
    assert!(html.contains("Axis2"));
    assert!(html.contains("Boost"));
    assert!(
        !html.contains("HATS"),
        "empty Hats group must not render header"
    );
}

#[test]
fn mapping_list_zero_mappings_renders_empty_state_a() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("No mappings yet"),
        "Empty State A must render when no mappings are present: {html}",
    );
}

#[test]
fn context_menu_renders_when_menu_open_is_set() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row"),
        "row must render so the contextmenu handler is bound: {html}",
    );
}

#[test]
fn delete_dialog_renders_when_target_set() {
    use crate::context::{GlyphFlags, MappingSummary};
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let target = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let ctx = use_context::<AppContext>();
        let delete_target = use_signal(|| {
            Some(crate::mapping_rail::Target {
                scope: crate::mapping_rail::Scope {
                    profile: ctx.meta.peek().profile_id.clone(),
                    mode: target.mode.clone(),
                },
                before: inputforge_core::action::Mapping {
                    input: target.input,
                    mode: target.mode,
                    name: target.name,
                    actions: vec![],
                },
            })
        });
        rsx! {
            crate::frame::mapping_list::DeleteDialogMount {
                delete_target: delete_target,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Boost"),
        "dialog must mention the row name: {html}"
    );
    assert!(
        html.contains("Delete") && html.contains("Cancel"),
        "dialog must show Delete + Cancel buttons: {html}",
    );
}

#[test]
fn duplicate_click_arms_live_capture() {
    use super::add_inline::AddInline;
    use crate::patterns::live_capture::LiveCapture;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        crate::test_support::engine()
            .state
            .write()
            .active_profile
            .as_mut()
            .unwrap()
            .set_mapping(
                &InputAddress::Bound {
                    device: DeviceId("dev".into()),
                    input: InputId::Button { index: 0 },
                },
                "Default",
                Some("Boost".into()),
                vec![],
            );
        let cap = use_context::<LiveCapture>();
        let force = use_signal(|| false);
        let duplicate = use_signal(|| {
            Some(crate::mapping_rail::Target {
                scope: crate::mapping_rail::Scope {
                    profile: ctx.meta.peek().profile_id.clone(),
                    mode: "Default".into(),
                },
                before: inputforge_core::action::Mapping {
                    input: InputAddress::Bound {
                        device: DeviceId("dev".into()),
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Default".into(),
                    name: Some("Boost".into()),
                    actions: vec![],
                },
            })
        });
        let armed = if *cap.active.read() { "ARMED" } else { "IDLE" };
        rsx! {
            AddInline { force_expanded: force, pending_duplicate: duplicate }
            span { "{armed}" }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.render_immediate_to_vec();
    let html = render(&vdom);
    assert!(
        html.contains("ARMED"),
        "actual duplicate mount must arm capture: {html}"
    );
    assert!(html.contains("Boost (copy)"));
}

#[test]
fn empty_zero_mappings_renders_full_anatomy() {
    use crate::frame::mapping_list::empty::EmptyZeroMappings;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroMappings {}
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("No mappings yet"), "title missing: {html}");
    assert!(
        html.contains("Click + Add mapping below to start one."),
        "helper text missing: {html}",
    );
    assert!(
        html.contains("if-rail-empty"),
        "rail-empty container class missing: {html}"
    );
}

#[test]
fn empty_zero_filter_results_renders_full_anatomy() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "ailerons".to_owned(),
                device_label: None,
                on_clear_text: move |()| {},
                on_clear_device: None,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ailerons"),
        "title must quote the filter query: {html}"
    );
    assert!(
        html.contains("Filter searches name and source label."),
        "exact helper text per spec missing: {html}",
    );
    assert!(
        html.contains("Clear text"),
        "Clear text ghost-link missing: {html}"
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "seeded-snapshot SSR test inlines a 4-mapping fixture covering MapToVJoy, \
              MergeAxis, Conditional, and resting Button, splitting it into helpers \
              hurts readability for a test whose value is the whole assembled fixture."
)]
fn rail_with_seeded_snapshot_renders_groups_rows_and_glyphs() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis,
    };

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        let mappings = vec![
            Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Throttle".to_owned()),
                actions: vec![Action::MapToVJoy {
                    output: OutputAddress {
                        device: 1,
                        output: OutputId::Axis { id: VJoyAxis::X },
                    },
                }],
            },
            Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                },
                mode: "Default".to_owned(),
                name: Some("Yaw".to_owned()),
                actions: vec![Action::MergeAxis {
                    second_input: InputAddress::Bound {
                        device: DeviceId("dev".to_owned()),
                        input: InputId::Axis { index: 2 },
                    },
                    operation: MergeOp::Average,
                }],
            },
            Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 3 },
                },
                mode: "Default".to_owned(),
                name: Some("Pitch".to_owned()),
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: InputAddress::Bound {
                            device: DeviceId("dev".to_owned()),
                            input: InputId::Button { index: 5 },
                        },
                    },
                    if_true: vec![],
                    if_false: Vec::new(),
                }],
            },
            Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Boost".to_owned()),
                actions: vec![],
            },
        ];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);

    let axes_pos = html.find("AXES").expect("AXES header missing");
    let buttons_pos = html.find("BUTTONS").expect("BUTTONS header missing");
    assert!(axes_pos < buttons_pos, "AXES must render before BUTTONS");

    assert!(html.contains("Throttle"));
    assert!(html.contains("Yaw"));
    assert!(html.contains("Pitch"));
    assert!(html.contains("Boost"));

    assert!(
        html.contains("glyph-merge"),
        "MergeAxis row must render gold + glyph"
    );
    assert!(
        html.contains("glyph-cond"),
        "Conditional row must render violet glyph"
    );
}

#[test]
fn active_row_carries_is_active_class_in_full_rail() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let target_input = InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: target_input.clone(),
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let view = use_context::<crate::frame::view_state::ViewState>();
        let mut sel = view.selected_mapping;
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
            sel.set(Some(("Default".to_owned(), target_input)));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains(r#"data-selected="true""#),
        "selected row must render is-active in the full rail; got: {html}",
    );
}

#[test]
fn inline_rename_swaps_in_for_active_row() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let target_input = InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        };
        rsx! {
            crate::frame::mapping_list::rename_inline::RenameInline {
                summary: crate::context::MappingSummary {
                    input: target_input.clone(),
                    mode: "Default".to_owned(),
                    name: Some("Boost".to_owned()),
                    glyphs: crate::context::GlyphFlags::default(),
                    referenced_devices: vec![DeviceId("dev".to_owned())],
                    first_vjoy_output: None,
                },
                state: use_signal(|| Some(target_input)),
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "rename-inline class must be present when state is Some: {html}",
    );
}

#[test]
fn colors_css_declares_tint_selected_and_tint_create() {
    let css = include_str!("../../../assets/tokens/colors.css");
    assert!(
        css.contains("--tint-selected: 8%;"),
        "--tint-selected token must be declared so the rail row, device chip, and \
         create-row hover can color-mix from one source: {css}",
    );
    assert!(
        css.contains("--tint-create:   5%;"),
        "--tint-create token must be declared so the dashed footer hover reads \
         as create rather than selected: {css}",
    );
}

#[test]
fn mode_tabs_active_tab_renders_canonical_if_tab_active_class() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        // `provide_minimal_contexts` supplies AppContext, ViewState,
        // ToastQueue, and live-capture. ModeTabs additionally needs
        // ModeDeleteSignal, which we provide inline below.
        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        // ModeDeleteSignal + ModeFocusSignal are provided shell-side
        // normally; provide local stubs so ModeTabs can mount in
        // isolation.
        let dt = use_signal(|| None);
        let mf: Signal<Option<String>> = use_signal(|| None);
        crate::frame::top_bar::mode_tabs::provide_publication(dt, mf);
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-tab--active"),
        "mode tabs must use the canonical .if-tab--active underline class: {html}",
    );
    assert!(
        !html.contains("if-mode-tab--active"),
        "legacy hand-rolled .if-mode-tab--active class must be retired: {html}",
    );
}

#[test]
fn mode_tabs_add_button_lives_outside_tablist() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let dt = use_signal(|| None);
        let mf: Signal<Option<String>> = use_signal(|| None);
        crate::frame::top_bar::mode_tabs::provide_publication(dt, mf);
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let tablist_open = html.find("role=\"tablist\"").expect("tablist must render");
    let tablist_close_relative = html[tablist_open..].find("</div>").expect("tablist closes");
    let tablist_close = tablist_open + tablist_close_relative;
    let plus_idx = html
        .find("aria-label=\"Add mode\"")
        .expect("Add mode button must render");
    assert!(
        plus_idx > tablist_close,
        "Add-mode `+` must render OUTSIDE the role=tablist container so AT \
         tab counts stay honest. tablist_close={tablist_close}, plus_idx={plus_idx}",
    );
}

#[test]
fn mode_tabs_running_pip_uses_canonical_class() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        // Force the runtime mode to match the only tab so the marker
        // resolves to tab_index = Some(0).
        state.current_mode = "Default".to_owned();

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let dt = use_signal(|| None);
        let mf: Signal<Option<String>> = use_signal(|| None);
        crate::frame::top_bar::mode_tabs::provide_publication(dt, mf);
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-tab__running-pip"),
        "running tab must render the canonical .if-tab__running-pip class so \
         the live-mode marker is shared with future Tabs primitive consumers: {html}",
    );
    assert!(
        !html.contains("if-mode-tab__marker"),
        "legacy bespoke .if-mode-tab__marker class must be retired: {html}",
    );
}

#[test]
fn group_header_renders_post_filter_row_count() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        let mut mappings = vec![];
        for i in 0..3 {
            mappings.push(Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: i },
                },
                mode: "Default".to_owned(),
                name: Some(format!("Axis{i}")),
                actions: vec![],
            });
        }
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let mut cfg_signal = use_context::<AppContext>().config;
        let mut meta_signal = use_context::<AppContext>().meta;
        use_hook(move || {
            cfg_signal.set(inputforge_app::bridge::prepare(&state, None).config);
            meta_signal.set(inputforge_app::bridge::prepare(&state, None).meta);
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail__group-header__count"),
        "group header must render a count slot class so the count reads as data: {html}",
    );
    assert!(
        html.contains("\"if-rail__group-header__count\">3"),
        "axes group with 3 mappings must show the count `3` inside the canonical group-header count slot: {html}",
    );
}

#[test]
fn mapping_list_css_aligns_group_header_gutter_with_row_padding() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__group-header {")
        .nth(1)
        .expect(".if-rail__group-header rule present")
        .split('}')
        .next()
        .expect(".if-rail__group-header rule closed");
    assert!(
        block.contains("padding: var(--space-3) var(--space-3) var(--space-1);"),
        "group header horizontal gutter must match the new row padding (--space-3): {block}",
    );
}

/// Rounded row corners read symmetrically only when both inline edges
/// reserve equal gutter space. Bare `scrollbar-gutter: stable` shipped
/// rows flush left vs ~10px-inset right, making the radius read
/// asymmetric. Lock `both-edges` so a future bare `stable` cannot
/// silently regress.
#[test]
fn mapping_list_css_locks_scroll_container_uses_stable_both_edges_gutter() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__scroll {")
        .nth(1)
        .expect(".if-rail__scroll rule present")
        .split('}')
        .next()
        .expect(".if-rail__scroll rule closed");
    assert!(
        block.contains("scrollbar-gutter: stable both-edges;"),
        ".if-rail__scroll must declare `scrollbar-gutter: stable both-edges` \
         so rows inset symmetrically and rounded corners read on both \
         sides: {block}",
    );
}

/// `--color-border-strong` is a border tier (#424766) and produced
/// ~1.5:1 contrast on the rail's --color-bg-elevated surface,
/// rendering the post-filter count practically invisible. Lock the
/// text-muted tier so the count stays legible alongside the eyebrow
/// label.
#[test]
fn mapping_list_css_locks_group_header_count_uses_text_tier() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__group-header__count {")
        .nth(1)
        .expect(".if-rail__group-header__count rule present")
        .split('}')
        .next()
        .expect(".if-rail__group-header__count rule closed");
    assert!(
        block.contains("color: var(--color-text-muted);"),
        ".if-rail__group-header__count must use --color-text-muted (a text \
         tier) so contrast stays legible; --color-border-strong is a \
         border tier and produced ~1.5:1 on the rail surface: {block}",
    );
    assert!(
        !block.contains("color: var(--color-border-strong);"),
        ".if-rail__group-header__count must NOT use --color-border-strong: {block}",
    );
}

/// The `SortableHandle` (6-dot grip) is `position: absolute; left: 4px`
/// and the row pays no left-padding gutter (Section E of the cohesion
/// spec drops it for density). Without a halo the hover-revealed
/// handle reads as glued to the labels behind it. Lock the chip-like
/// halo so a future regression cannot strip it back to a transparent
/// overlay.
#[test]
fn mapping_list_css_locks_handle_halo_on_row_hover() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-row:hover .if-sortable-handle {")
        .nth(1)
        .expect(".if-row:hover .if-sortable-handle rule present")
        .split('}')
        .next()
        .expect(".if-row:hover .if-sortable-handle rule closed");
    assert!(
        block.contains("background: var(--color-bg-elevated);"),
        "handle halo must use --color-bg-elevated so it matches the row's \
         hover background and reads as a chip-like surface: {block}",
    );
    assert!(
        block.contains("border: 1px solid var(--color-border);"),
        "handle halo must carry a 1 px --color-border hairline (chip idiom): {block}",
    );
    assert!(
        block.contains("border-radius: var(--radius-sm);"),
        "handle halo must use --radius-sm to match the dense-chip idiom: {block}",
    );
}

/// Mapping display and rename content share the same clearance beyond the hover grip.
#[test]
fn mapping_list_css_locks_source_line_gutter() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    for selector in [
        ".if-row__name",
        ".if-row__source",
        ".if-row__rename-wrapper",
    ] {
        let block = css
            .split(&format!("\n{selector} {{"))
            .nth(1)
            .unwrap_or_else(|| panic!("standalone {selector} rule present"))
            .split('}')
            .next()
            .expect("rule closed");
        assert!(
            block.contains("padding-left: var(--space-3);"),
            "{selector} must keep the full text gutter beyond the hovered grip: {block}",
        );
    }
}

/// `row.rs:82` previously discarded `split_label`'s input half. Lock
/// the contract that all three input kinds (Axis / Button / Hat) make
/// it onto the source-primary line, the device cell is followed by
/// the input cell, and the input cell text matches `split_label`'s
/// output (HID axis label, 1-indexed `Btn N`, 0-indexed `Hat N`).
#[test]
fn row_renders_input_identity_after_device_for_each_input_kind() {
    use inputforge_core::types::InputId;

    fn axis_test() -> Element {
        row_input_identity_test_component(InputId::Axis { index: 0 })
    }
    fn button_test() -> Element {
        row_input_identity_test_component(InputId::Button { index: 3 })
    }
    fn hat_test() -> Element {
        row_input_identity_test_component(InputId::Hat { index: 0 })
    }

    assert_input_cell_renders(axis_test, "X", "axis 0");
    assert_input_cell_renders(button_test, "Btn 4", "button index 3");
    assert_input_cell_renders(hat_test, "Hat 0", "hat 0");
}

fn row_input_identity_test_component(input: inputforge_core::types::InputId) -> Element {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress,
    };

    provide_minimal_contexts();
    let ctx = use_context::<AppContext>();
    let mut cfg_signal = ctx.config;
    use_hook(move || {
        cfg_signal.set(ConfigSnapshot {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId("dev".to_owned()),
                    name: "TFM Throttle".to_owned(),
                    axes: 4,
                    buttons: 8,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar; 4],
                },
                connected: true,
                diagnostics: DeviceDiagnostics::default(),
            }],
            ..ConfigSnapshot::default()
        });
    });
    let summary = MappingSummary {
        input: InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input,
        },
        mode: "Default".to_owned(),
        name: None,
        glyphs: GlyphFlags::default(),
        referenced_devices: vec![DeviceId("dev".to_owned())],
        first_vjoy_output: None,
    };
    let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
    let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
    rsx! {
        Row {
            summary: summary,
            is_active: false,
            renaming: renaming,
            sortable: sortable,
            filter_active: false,
            on_open_menu: move |_: (InputAddress, f64, f64)| {},
        }
    }
}

fn assert_input_cell_renders(component: fn() -> Element, expected_text: &str, kind_label: &str) {
    let mut vdom = VirtualDom::new(component);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    let device_pos = html
        .find("if-row__source-device")
        .unwrap_or_else(|| panic!("device cell missing for {kind_label}: {html}"));
    let input_pos = html
        .find("if-row__source-input")
        .unwrap_or_else(|| panic!("input cell missing for {kind_label}: {html}"));
    assert!(
        device_pos < input_pos,
        "source-primary cell order must be device then input \
         for {kind_label}: {html}",
    );
    assert!(
        html.contains(&format!(">{expected_text}<")),
        "input cell must render `{expected_text}` for {kind_label}: {html}",
    );
}

#[test]
fn qualifier_chips_render_as_chip_outline_with_glyph_class() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-chip--outline"),
        "qualifier chips must use Chip Outline variant: {html}",
    );
    assert!(
        html.contains("glyph-merge"),
        "merge glyph class must remain so the leading glyph keeps its --color-output hue: {html}",
    );
    assert!(
        html.contains("glyph-cond"),
        "conditional glyph class must remain so the leading glyph keeps its --color-control-badge-text hue: {html}",
    );
    assert!(
        !html.contains("if-row__chip\""),
        "legacy .if-row__chip class (without the chip-glyph suffix) must be retired: {html}",
    );
}

/// Row-to-row spacing is owned by `.if-sortable-gap` (height
/// `var(--space-2)`); the previous bare 2 px gap on the group
/// container stacked on top and shipped ~12 px of dead air per row,
/// missing the 4-px-grid scale. Lock the gap-free group container so
/// the only contributor is the sortable gap.
#[test]
fn mapping_list_css_locks_rail_group_drops_inter_row_flex_gap() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__group {")
        .nth(1)
        .expect(".if-rail__group rule present")
        .split('}')
        .next()
        .expect(".if-rail__group rule closed");
    let block_no_comments = strip_css_block_comments(block);
    assert!(
        block_no_comments.contains("display: flex;")
            && block_no_comments.contains("flex-direction: column;"),
        ".if-rail__group must remain a column flex container so the \
         sortable gap stacks vertically: {block_no_comments}",
    );
    assert!(
        !block_no_comments.contains("gap:"),
        ".if-rail__group must not declare a `gap` property; row-to-row \
         spacing is owned by .if-sortable-gap height: {block_no_comments}",
    );
}

/// Helper: strip `/* ... */` comment spans from a CSS block so contract
/// assertions see only declarations, not commentary that may quote
/// historical values for context.
fn strip_css_block_comments(block: &str) -> String {
    let mut out = String::with_capacity(block.len());
    let mut chars = block.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(d) = chars.next() {
                if d == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn device_filter_active_chip_emits_unified_active_class() {
    use crate::context::{GlyphFlags, MappingSummary};
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg_signal = ctx.config;
        let view = use_context::<crate::frame::view_state::ViewState>();
        let _ = view;
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot {
                devices: vec![DeviceState {
                    info: DeviceInfo {
                        id: DeviceId("stick".to_owned()),
                        name: "Twin Stick".to_owned(),
                        axes: 1,
                        buttons: 1,
                        hats: 0,
                        instance_path: None,
                        axis_polarities: vec![AxisPolarity::Bipolar],
                    },
                    connected: true,
                    diagnostics: DeviceDiagnostics::default(),
                }],
                mappings: vec![MappingSummary {
                    input: InputAddress::Bound {
                        device: DeviceId("stick".to_owned()),
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Default".to_owned(),
                    name: Some("Boost".to_owned()),
                    glyphs: GlyphFlags::default(),
                    referenced_devices: vec![DeviceId("stick".to_owned())],
                    first_vjoy_output: None,
                }],
                device_display_names: std::collections::HashMap::from([(
                    DeviceId("stick".to_owned()),
                    "Twin Stick".to_owned(),
                )]),
                ..ConfigSnapshot::default()
            });
        });
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-toggle-chip"),
        "device filter chip must render the canonical .if-chip class \
         (Chip primitive): {html}",
    );
    assert!(html.contains(r#"aria-pressed="false""#), "{html}");
    // The wrapping button keeps the .if-rail__device-chip class as a CSS
    // hook for the click target reset; the chip's visual chrome lives on
    // the inner .if-chip element. The legacy active-state class
    // .if-rail__device-chip.is-active is what we are retiring.
    assert!(
        !html.contains("if-rail__device-chip is-active")
            && !html.contains("if-rail__device-chip.is-active"),
        "legacy .if-rail__device-chip.is-active active-state class must be retired: {html}",
    );
}

#[test]
fn mapping_list_css_wraps_device_filter_chips_into_multiple_rows() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__device-filter {")
        .nth(1)
        .expect(".if-rail__device-filter rule present")
        .split('}')
        .next()
        .expect(".if-rail__device-filter rule closed");
    assert!(
        block.contains("flex-wrap: wrap;"),
        "device filter strip must wrap to a multi-row layout (no scroll-x); got: {block}",
    );
    assert!(
        !block.contains("overflow-x: auto;"),
        "device filter strip must NOT use overflow-x scrolling after the wrap migration; got: {block}",
    );
}

#[test]
fn row_output_chip_replaces_legacy_output_badge() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Pitch".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: Some(OutputAddress {
                device: 2,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        !html.contains("if-row__output-badge"),
        "legacy .if-row__output-badge class must NOT render after the migration: {html}",
    );
    let chip_count = html.matches("if-chip--output").count();
    assert_eq!(
        chip_count, 1,
        "row with first_vjoy_output must render exactly one .if-chip--output element; got {chip_count} in: {html}",
    );
    assert!(
        html.contains("\u{2192}"),
        "source line must include the arrow glyph U+2192 separating trigger from output: {html}",
    );
    assert!(
        html.contains("aria-hidden=\"true\""),
        "arrow glyph must be aria-hidden so screen readers rely on label sequence: {html}",
    );
    assert!(
        html.contains("if-row__output-chip-text"),
        "output chip text wrapper missing: {html}"
    );
    assert!(
        html.contains("title=\"Device 2 · X\""),
        "full compact address must remain available as the chip title: {html}"
    );
}

#[test]
fn mapping_list_css_bounds_and_ellipsizes_output_chip() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let chip = css
        .split(".if-row__output-chip {")
        .nth(1)
        .expect("output chip rule present")
        .split('}')
        .next()
        .expect("output chip rule closed");
    let text = css
        .split(".if-row__output-chip-text {")
        .nth(1)
        .expect("output chip text rule present")
        .split('}')
        .next()
        .expect("output chip text rule closed");

    for declaration in ["flex: 0 1 auto;", "min-width: 0;", "max-width: 50%;"] {
        assert!(chip.contains(declaration), "missing {declaration}: {chip}");
    }
    for declaration in [
        "display: block;",
        "min-width: 0;",
        "overflow: hidden;",
        "text-overflow: ellipsis;",
        "white-space: nowrap;",
    ] {
        assert!(text.contains(declaration), "missing {declaration}: {text}");
    }
}

/// Negative half of `row_output_chip_replaces_legacy_output_badge`:
/// when `first_vjoy_output: None`, the row must render no
/// `.if-chip--output` element and no arrow glyph. Locks the other half
/// of the conditional render at `row.rs:238` so a future regression
/// that always renders the chip (or the separator) fails fast.
#[test]
fn row_output_chip_absent_when_no_vjoy() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Pitch".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let chip_count = html.matches("if-chip--output").count();
    assert_eq!(
        chip_count, 0,
        "row without first_vjoy_output must render zero .if-chip--output elements; got {chip_count} in: {html}",
    );
    assert!(
        !html.contains("\u{2192}"),
        "no arrow glyph U+2192 must paint when there is no output chip to separate to: {html}",
    );
}
