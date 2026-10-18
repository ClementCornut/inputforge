//! Regressions for named feedback, disclosure and empty content surfaces.

use dioxus::prelude::*;
use dioxus_ssr::render;

use crate::components::{
    Cluster, Disclosure, EmptyState, FeedbackTone, InlineFeedback, ReadoutCaption, Separator,
    SeparatorOrientation, StatusDot,
};

fn html(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    render(&dom)
}

#[test]
fn feedback_keeps_stable_error_reference_without_forcing_alerts() {
    fn fixture() -> Element {
        rsx! {
            input { "aria-describedby": "alias-error", "aria-invalid": "true" }
            InlineFeedback { id: "alias-error", tone: FeedbackTone::Error, "Save failed" }
        }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("id=\"alias-error\""));
    assert!(rendered.contains("aria-describedby=\"alias-error\""));
    assert!(rendered.contains("if-inline-feedback--error"));
    assert!(!rendered.contains("--floating"));
    assert!(!rendered.contains("role=\"alert\""));
}

#[test]
fn workflow_alerts_and_floating_feedback_are_deliberate() {
    fn fixture() -> Element {
        rsx! { InlineFeedback { alert: true, floating: true, "Dispatch failed" } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("role=\"alert\""));
    assert!(rendered.contains("if-inline-feedback--floating"));
    assert!(!rendered.contains("id="));
}

#[test]
fn disclosure_preserves_native_collapsed_state_and_summary() {
    fn fixture() -> Element {
        rsx! { Disclosure { summary: rsx! { "Advanced" }, p { "Settings" } } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("<details"));
    assert!(rendered.contains("<summary"));
    assert!(rendered.contains("Advanced</summary>"));
    assert!(rendered.contains("Settings"));
    assert!(!rendered.contains("open="));
    assert!(!rendered.contains("role="));
    assert!(!rendered.contains("aria-expanded="));
}

#[test]
fn empty_state_composes_content_and_actions_without_workflow() {
    fn fixture() -> Element {
        rsx! {
            EmptyState {
                title: "No profile", description: "Open or create one",
                eyebrow: rsx! { ReadoutCaption { "Standby" } },
                actions: rsx! { button { "Open" } },
                p { "Additional guidance" }
            }
        }
    }
    let rendered = html(fixture);
    for text in [
        "No profile",
        "Open or create one",
        "Standby",
        "Open",
        "Additional guidance",
    ] {
        assert!(rendered.contains(text));
    }
    assert!(rendered.contains("<h2"));
}

#[test]
fn decorative_status_dots_and_separators_do_not_add_semantics() {
    fn fixture() -> Element {
        rsx! { StatusDot {} Separator { decorative: true, strong: true } }
    }
    let rendered = html(fixture);
    assert!(!rendered.contains("role="));
    assert_eq!(rendered.matches("aria-hidden=\"true\"").count(), 2);
    assert!(rendered.contains("if-separator--strong"));
}

#[test]
fn vertical_separator_and_named_dot_have_correct_semantics() {
    fn fixture() -> Element {
        rsx! {
            Separator { orientation: SeparatorOrientation::Vertical }
            StatusDot { label: "Paused", hollow: true }
        }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("role=\"separator\""));
    assert!(rendered.contains("aria-orientation=\"vertical\""));
    assert!(rendered.contains("role=\"img\""));
    assert!(rendered.contains("aria-label=\"Paused\""));
    assert!(rendered.contains("if-status-dot--hollow"));
}

#[test]
fn inline_nowrap_cluster_remains_a_content_group() {
    fn fixture() -> Element {
        rsx! { Cluster { inline: true, nowrap: true, span { "Input" } span { "Output" } } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("if-cluster--inline"));
    assert!(rendered.contains("if-cluster--nowrap"));
    assert!(!rendered.contains("role="));
}
