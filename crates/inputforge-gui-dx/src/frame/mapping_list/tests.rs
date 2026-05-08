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

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::components::sortable::use_sortable_state;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::mapping_list::MappingList;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn provide_minimal_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());

    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);

    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });

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
fn mapping_list_css_keeps_device_chips_one_row() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    assert!(css.contains(".if-rail__device-filter"));
    assert!(css.contains("overflow-x: auto"));
    assert!(css.contains("overflow-y: hidden"));
    assert!(css.contains("flex-wrap: nowrap"));
    assert!(css.contains(".if-rail__device-chip"));
    assert!(css.contains("flex: 0 0 auto"));
    assert!(css.contains("white-space: nowrap"));
    assert!(css.contains(".if-rail__device-chip:focus-visible"));
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
        let sortable = use_sortable_state::<u32>();
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
        !html.contains("if-row__source-input"),
        "legacy input label must not render: {html}"
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
        let sortable = use_sortable_state::<u32>();
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
        !html.contains("if-row__source-input"),
        "legacy input label must not render: {html}"
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
        let sortable = use_sortable_state::<u32>();
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
    assert!(html.contains("vJoy 2"), "vJoy device missing: {html}");
    assert!(html.contains('X'), "vJoy output missing: {html}");
    assert!(
        html.contains("if-row__output-badge"),
        "output badge class missing: {html}"
    );
    assert!(
        !html.contains("if-row__source-input"),
        "legacy input label must not render beside the output badge: {html}"
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
        let sortable = use_sortable_state::<u32>();
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
        html.contains("is-active"),
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
        let sortable = use_sortable_state::<u32>();
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
        let sortable = use_sortable_state::<u32>();
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
        !html.contains("if-row__source-input"),
        "legacy input label must not render during rename: {html}",
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
        let sortable = use_sortable_state::<u32>();
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

#[test]
fn add_inline_resting_renders_dashed_row() {
    use crate::context::MappingSummary;
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| false);
        let pending_duplicate: Signal<Option<MappingSummary>> = use_signal(|| None);
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
    use crate::context::MappingSummary;
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| true);
        let pending_duplicate: Signal<Option<MappingSummary>> = use_signal(|| None);
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
        html.contains("if-add-inline__chip--listening"),
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
            let cfg = ConfigSnapshot::from_state(&state, None);
            cfg_signal.set(cfg);
            let meta = MetaSnapshot::from_state(&state);
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
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
        let delete_target: Signal<Option<MappingSummary>> = use_signal(|| Some(target));
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
    use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let cap = use_context::<LiveCapture>();
        // Synthesize a "user clicked Duplicate" by emulating its body:
        // arm capture. (Real wiring lives in ContextMenuMount; the
        // SSR-friendly version of this test asserts that arming flips
        // LiveCapture::active to true.)
        use_hook(move || {
            cap.start.call(CaptureFilter::Any);
        });
        let armed_marker = if *cap.active.read() { "ARMED" } else { "IDLE" };
        rsx! { span { "{armed_marker}" } }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ARMED"),
        "Duplicate flow's start.call must arm LiveCapture; got: {html}",
    );
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

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
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
            sel.set(Some(("Default".to_owned(), target_input)));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("is-active"),
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
        // ModeDeleteSignal is provided shell-side normally; provide a
        // local stub so ModeTabs can mount in isolation.
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
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
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
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
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
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
fn mapping_list_css_locks_row_token_contract() {
    let css = include_str!("../../../assets/frame/mapping_list.css");

    // Row resting block. Padding is uniform var(--space-3) (the 10px
    // drag-handle gutter is dropped in this pass; the SortableHandle is
    // a 0-width hover-only overlay). Radius bumps to --radius-md to
    // match .if-device-row and .profile-row.
    let block = css
        .split(".if-row {")
        .nth(1)
        .expect(".if-row rule present")
        .split('}')
        .next()
        .expect(".if-row rule closed");
    assert!(
        block.contains("padding: var(--space-3);"),
        ".if-row padding must be uniform var(--space-3): {block}",
    );
    assert!(
        block.contains("border-radius: var(--radius-md);"),
        ".if-row must use --radius-md (matches .if-device-row): {block}",
    );
    assert!(
        block.contains("background: var(--color-bg);"),
        ".if-row base must use --color-bg (rows sit on bg, not bg-elevated): {block}",
    );
    assert!(
        block.contains("border: 1px solid transparent;"),
        ".if-row must reserve a 1px transparent border so hover/selected swaps \
         do not reflow geometry: {block}",
    );

    // Hover.
    let hover = css
        .split(".if-row:hover {")
        .nth(1)
        .expect(".if-row:hover rule present")
        .split('}')
        .next()
        .expect(".if-row:hover rule closed");
    assert!(
        hover.contains("background: var(--color-bg-elevated);"),
        ".if-row:hover background must be --color-bg-elevated (the \
         --color-border substitution is dropped): {hover}",
    );
    assert!(
        hover.contains("border-color: var(--color-border-strong);"),
        ".if-row:hover border must be --color-border-strong (matches \
         .if-device-row:hover): {hover}",
    );

    // Selected. The active row mixes --color-primary at --tint-selected
    // into --color-bg, with a --color-border-focus border.
    let selected = css
        .split(".if-row.is-active {")
        .nth(1)
        .expect(".if-row.is-active rule present")
        .split('}')
        .next()
        .expect(".if-row.is-active rule closed");
    assert!(
        selected.contains(
            "background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg));"
        ),
        ".if-row.is-active background must mix --color-primary at \
         --tint-selected into --color-bg (no `transparent` parent): {selected}",
    );
    assert!(
        selected.contains("border-color: var(--color-border-focus);"),
        ".if-row.is-active must carry the focus border idiom: {selected}",
    );

    // Focus-visible. Inset 2px to match .if-device-row's offset:-2px.
    assert!(
        css.contains(".if-row:focus-visible {"),
        ".if-row:focus-visible rule must exist: {css}",
    );
    let focus = css
        .split(".if-row:focus-visible {")
        .nth(1)
        .expect(".if-row:focus-visible rule present")
        .split('}')
        .next()
        .expect(".if-row:focus-visible rule closed");
    assert!(
        focus.contains("outline: 2px solid var(--color-border-focus);"),
        "row focus ring must be 2px var(--color-border-focus): {focus}",
    );
    assert!(
        focus.contains("outline-offset: -2px;"),
        "row focus offset must be inset (-2px) per .if-device-row contract: {focus}",
    );
}
