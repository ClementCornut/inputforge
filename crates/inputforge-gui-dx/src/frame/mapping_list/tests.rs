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
