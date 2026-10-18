//! Shared feedback accessibility contracts.

use dioxus::prelude::*;
use dioxus_ssr::render;

use super::{badge::Badge, chip::Chip, tooltip::Tooltip};

fn html(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    render(&dom)
}

#[test]
fn tooltip_trigger_explicitly_references_only_its_bubble() {
    fn fixture() -> Element {
        rsx! {
            Tooltip { id: "curve-help", content: "Keyboard instructions",
                button { "aria-describedby": "curve-help", "Curve" }
            }
            Tooltip { id: "zone-help", content: "Zone instructions",
                button { "aria-describedby": "zone-help", "Zone" }
            }
        }
    }
    let rendered = html(fixture);
    assert_eq!(rendered.matches("id=\"curve-help\"").count(), 1);
    assert_eq!(rendered.matches("id=\"zone-help\"").count(), 1);
    assert!(rendered.contains("aria-describedby=\"curve-help\""));
    assert!(rendered.contains("aria-describedby=\"zone-help\""));
    assert_eq!(rendered.matches("role=\"tooltip\"").count(), 2);
}

#[test]
fn tooltip_without_id_does_not_invent_a_trigger_association() {
    fn fixture() -> Element {
        rsx! { Tooltip { content: "Hint", span { "Static" } } }
    }
    let rendered = html(fixture);
    assert!(!rendered.contains("id="));
    assert!(!rendered.contains("aria-describedby="));
}

#[test]
fn counts_do_not_create_live_regions_inside_notifications() {
    fn fixture() -> Element {
        rsx! { Badge { count: true, "3" } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("if-badge--count"));
    assert!(!rendered.contains("aria-live="));
    assert!(!rendered.contains("role="));
}

#[test]
fn badge_live_announcements_are_explicit() {
    fn fixture() -> Element {
        rsx! { Badge { aria_live: "polite", aria_label: "Three connected devices", "3" } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("aria-live=\"polite\""));
    assert!(rendered.contains("aria-label=\"Three connected devices\""));
}

#[test]
fn capture_chip_carries_its_own_input_taxonomy() {
    fn fixture() -> Element {
        rsx! { Chip { variant: super::chip::ChipVariant::Capture, kind: "hat", "HAT 1" } }
    }
    let rendered = html(fixture);
    assert!(rendered.contains("data-kind=\"hat\""));
    assert!(rendered.contains("if-chip--capture"));
    assert!(!rendered.contains("aria-live="));
}
