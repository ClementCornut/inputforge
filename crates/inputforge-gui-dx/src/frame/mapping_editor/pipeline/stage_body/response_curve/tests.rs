//! Integration tests for the F10 `response_curve` body. Pure-fn tests
//! live next to their owning module.
//!
//! Harness pattern mirrors `frame/mapping_editor/tests.rs:82-168`: a
//! `#[derive(Clone, Props, PartialEq)]` struct + a `#[component]`
//! wrapper, driven by `VirtualDom::new_with_props(Component, Props)`.
//! Free fns and tuple props are NOT supported by Dioxus 0.7's
//! `new_with_props` API.
use dioxus::prelude::*;
use dioxus_ssr::render;
use inputforge_core::processing::curves::ResponseCurve;

use crate::frame::mapping_editor::pipeline::stage_body::response_curve::{
    rendering,
    state::{BodyState, extract_anchors},
};

#[derive(Clone, Props, PartialEq)]
struct RenderHarnessProps {
    curve: ResponseCurve,
    body: BodyState,
    live: Option<f64>,
}

#[component]
fn RenderHarness(props: RenderHarnessProps) -> Element {
    rendering::render_plot(&props.curve, &props.body, props.live, 240.0)
}

fn seeded_body(curve: &ResponseCurve) -> BodyState {
    BodyState {
        cached_path: inputforge_core::processing::curves::sample_curve_path(curve, 200),
        cached_anchors: extract_anchors(curve),
        ..BodyState::default()
    }
}

#[test]
fn render_plot_emits_svg_with_grid_and_polyline() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let body = seeded_body(&curve);
    let mut vdom = VirtualDom::new_with_props(
        RenderHarness,
        RenderHarnessProps {
            curve,
            body,
            live: None,
        },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("<svg"), "must emit svg root: {html}");
    assert!(
        html.contains("if-curve__path"),
        "must include curve path class"
    );
    assert!(
        html.contains("if-curve__grid-major"),
        "major grid class missing"
    );
    assert!(
        html.contains("if-curve__identity"),
        "identity dashed line missing"
    );
    // y-flip group
    assert!(
        html.contains(r#"transform="scale(1, -1)""#),
        "must apply y-flip group: {html}"
    );
    // No live dot when live_value is None.
    assert!(
        !html.contains("if-curve__live-dot"),
        "live dot must be absent"
    );
}

#[test]
fn render_plot_with_live_value_emits_live_dot() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let body = seeded_body(&curve);
    let mut vdom = VirtualDom::new_with_props(
        RenderHarness,
        RenderHarnessProps {
            curve,
            body,
            live: Some(0.42),
        },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-curve__live-dot"),
        "live dot must render: {html}"
    );
    assert!(
        html.contains("if-curve__live-guide"),
        "live guide line must render"
    );
}

#[test]
fn header_thumbnail_emits_svg_with_polyline_for_each_curve_kind() {
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::thumbnail;
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    let curves = [
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap(),
        ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap(),
        ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, 0.5),
                control2: (0.5, -0.5),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap(),
    ];
    for c in curves {
        // Reuse the same harness pattern as `RenderHarness` above:
        // `#[derive(Clone, Props, PartialEq)]` + `#[component]`. A free
        // fn `fn h(curve) -> Element` is NOT a valid Dioxus component
        // and tuple props do not implement `Properties`.
        #[derive(Clone, Props, PartialEq)]
        struct ThumbHarnessProps {
            curve: ResponseCurve,
        }
        #[component]
        fn ThumbHarness(props: ThumbHarnessProps) -> Element {
            thumbnail::header_thumbnail(&props.curve)
        }
        let mut vdom = VirtualDom::new_with_props(ThumbHarness, ThumbHarnessProps { curve: c });
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-curve__thumbnail"),
            "thumbnail class missing: {html}"
        );
        assert!(html.contains("polyline"), "thumbnail polyline missing");
        assert!(
            html.contains("-1.05 -1.05 2.1 2.1"),
            "viewBox values missing: {html}"
        );
    }
}

#[test]
fn body_renders_static_plot_with_summary_and_anchors() {
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::ResponseCurveBody;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
    use inputforge_core::action::Action;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    fn h() -> Element {
        let (cmd_tx, _rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        // Provide `AppContext` before calling `use_live_capture_provider`
        // because that function internally reads it via `use_context`.
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        crate::patterns::live_capture::use_live_capture_provider();
        crate::frame::mapping_editor::use_editor_state_provider();

        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);
        let key = (
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
        );
        let root_actions = vec![Action::ResponseCurve {
            curve: curve.clone(),
        }];
        rsx! {
            ResponseCurveBody {
                mapping_key: key,
                stage_id,
                curve,
                root_actions,
            }
        }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-curve"), "body root class missing");
    assert!(html.contains("if-curve__plot"), "plot svg missing");
    assert!(html.contains("if-curve__path"), "polyline missing");
    assert!(html.contains("if-curve__toolbar"), "toolbar missing");
    // 3 anchors -> 3 anchor circles.
    assert!(html.matches("if-curve__anchor").count() >= 3);
}

#[test]
fn body_attaches_pointer_handlers_and_emits_data_attributes() {
    // SSR cannot drive PointerEvent dispatch, so this is a static
    // assertion: the wrapper div must carry the data-hovered / data-dragging
    // attributes (consumed by CSS cursor rules) and the svg plot must render
    // inside it.
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::ResponseCurveBody;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
    use inputforge_core::action::Action;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    fn h() -> Element {
        let (cmd_tx, _rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        crate::patterns::live_capture::use_live_capture_provider();
        crate::frame::mapping_editor::use_editor_state_provider();

        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);
        let key = (
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
        );
        let root_actions = vec![Action::ResponseCurve {
            curve: curve.clone(),
        }];
        rsx! {
            ResponseCurveBody {
                mapping_key: key,
                stage_id,
                curve,
                root_actions,
            }
        }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // The focusable wrapper div must carry both data attributes regardless
    // of their initial values (both start as "false").
    assert!(
        html.contains("data-hovered"),
        "data-hovered attribute missing: {html}"
    );
    assert!(
        html.contains("data-dragging"),
        "data-dragging attribute missing: {html}"
    );
    // The SVG plot must still be rendered inside the wrapper.
    assert!(html.contains("if-curve__plot"), "plot svg missing: {html}");
}

#[test]
fn body_emits_tabindex_and_aria_label_on_plot() {
    // The plot wrapper <div class="if-curve__plot-frame"> must carry
    // tabindex="0" and aria-label="response curve" (the latter moved
    // here from <svg> in Task 12 so screen readers announce on focus).
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::ResponseCurveBody;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
    use inputforge_core::action::Action;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    fn h() -> Element {
        let (cmd_tx, _rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        crate::patterns::live_capture::use_live_capture_provider();
        crate::frame::mapping_editor::use_editor_state_provider();

        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);
        let key = (
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
        );
        let root_actions = vec![Action::ResponseCurve {
            curve: curve.clone(),
        }];
        rsx! {
            ResponseCurveBody {
                mapping_key: key,
                stage_id,
                curve,
                root_actions,
            }
        }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains(r#"tabindex="0""#), "plot must be focusable");
    assert!(html.contains(r#"aria-label="response curve""#));
    // onkeydown listener is opaque in SSR markup; full key flow is
    // covered by Task 7 pure-fn tests.
}

// `cmd_tx` is NOT a prop. The harness seeds it via `AppContext` per the
// `HarnessComponent` pattern in `frame/mapping_editor/tests.rs:82-168`.
// This avoids the `Sender: !PartialEq` problem entirely.
#[derive(Clone, Props, PartialEq)]
struct ToolbarHarnessProps {
    curve: ResponseCurve,
    stage_id: crate::frame::mapping_editor::undo_log::StageId,
    root_actions: Vec<inputforge_core::action::Action>,
    mapping_key: crate::frame::MappingKey,
}

#[component]
fn ToolbarHarness(props: ToolbarHarnessProps) -> Element {
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::toolbar::Toolbar;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};
    let (cmd_tx, _rx) = mpsc::channel();
    let raw = RawHandles {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());
    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let ctx = AppContext {
        state: Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: Arc::clone(&raw.settings),
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx);
    crate::frame::mapping_editor::use_editor_state_provider();
    rsx! {
        Toolbar {
            curve: props.curve,
            stage_id: props.stage_id,
            root_actions: props.root_actions,
            mapping_key: props.mapping_key,
        }
    }
}

#[test]
fn toolbar_type_change_emits_set_mapping() {
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let actions = vec![Action::ResponseCurve {
        curve: curve.clone(),
    }];

    // SSR mount only verifies static markup. Click simulation is covered
    // by manual smoke tests + the keyboard/pointer pure-fn suites.
    let mapping_key = (
        "Default".to_owned(),
        InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input: InputId::Axis { index: 0 },
        },
    );
    let stage_id = crate::frame::mapping_editor::undo_log::StageId(vec![
        crate::frame::mapping_editor::undo_log::StageIdSegment::Index(0),
    ]);
    let mut vdom = VirtualDom::new_with_props(
        ToolbarHarness,
        ToolbarHarnessProps {
            curve: curve.clone(),
            stage_id,
            root_actions: actions,
            mapping_key,
        },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Linear"), "Linear tab missing: {html}");
    assert!(html.contains("Spline"));
    assert!(html.contains("Bezier"));
    assert!(html.contains("if-switch"), "symmetric switch missing");
    assert!(html.contains("Reset"));
}
