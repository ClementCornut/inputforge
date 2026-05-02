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
