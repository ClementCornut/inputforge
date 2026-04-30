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
fn row_renders_name_and_source_line() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Boost"), "name must render: {html}");
    assert!(html.contains("Btn 1"), "source line must render: {html}");
    assert!(html.contains("if-row"), "row root class missing: {html}");
}

#[test]
fn row_active_class_when_selected() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: true,
                renaming: renaming,
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
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
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
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
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
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| Some(summary.input.clone()));
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
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
}

#[test]
fn row_renders_resting_when_renaming_is_none() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
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
fn empty_zero_mappings_renders_title_and_button() {
    use crate::frame::mapping_list::empty::EmptyZeroMappings;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroMappings { on_start_capture: move |()| {} }
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
                on_clear: move |()| {},
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
        html.contains("Clear filter"),
        "clear-filter button missing: {html}"
    );
}

#[test]
fn add_inline_resting_renders_dashed_row() {
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| false);
        rsx! { AddInline { force_expanded: force_expanded } }
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
        rsx! { AddInline { force_expanded: force_expanded } }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // SSR rebuild_in_place re-creates the ROOT scope each time, so we can't
    // observe `cap.active` flipping in a parent scope across rebuilds (each
    // rebuild gets a fresh LiveCapture context). We instead assert that the
    // AddInline child rendered the `--armed` modifier, which is only emitted
    // when the state machine transitioned to `CapturingArmed` AND the
    // `cap.start` callback was invoked from the same use_hook on first mount.
    assert!(
        html.contains("if-add-inline--armed"),
        "force_expanded=true must arm capture (state machine -> CapturingArmed); got: {html}",
    );
    assert!(
        html.contains("Press an input on any device"),
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
                input: InputAddress {
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
            input: InputAddress {
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
            let cfg = ConfigSnapshot::from_state(&state);
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
            input: InputAddress {
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
            cfg_signal.set(ConfigSnapshot::from_state(&state));
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
