// Rust guideline compliant 2026-05-03

//! SSR mount tests for `DeadzoneBody`. Mounts via the shared
//! `mount_stage_body_test` helper from Task 15.5; assertions target the
//! rendered HTML.
//!
//! Each test defines its own free `fn body() -> Element` rather than a
//! capturing closure: Dioxus 0.7's `VirtualDom::new` only accepts function
//! pointers, and `mount_stage_body_test` therefore takes `fn() -> Element`.
//! Free fns cannot capture, so per-test inputs (the `DeadzoneConfig`) are
//! constructed inline inside each `body`. Mirrors F10's
//! `response_curve/tests.rs` shape (no nested `mod tests` wrapper, since
//! the file is already named `tests.rs` and clippy's `module_inception`
//! lint forbids the redundant nesting).

use super::DeadzoneBody;
use dioxus::prelude::*;
use inputforge_core::action::Action;
use inputforge_core::processing::deadzone::DeadzoneConfig;
use inputforge_core::types::InputAddress;

use crate::frame::mapping_editor::test_helpers::mount_stage_body_test;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

#[test]
fn default_config_renders_4_handles() {
    fn body() -> Element {
        let key = ("default".to_owned(), InputAddress::Unbound);
        let sid = StageId(vec![StageIdSegment::Index(0)]);
        let cfg = DeadzoneConfig::default();
        rsx! {
            DeadzoneBody {
                mapping_key: key,
                stage_id: sid,
                config: cfg.clone(),
                root_actions: vec![Action::Deadzone { config: cfg }],
            }
        }
    }
    let html = mount_stage_body_test(body);
    let count = html.matches("if-deadzone__handle").count();
    assert_eq!(count, 4, "must render 4 handles, got {count}: {html}");
}

#[test]
fn renders_zone_band_classes() {
    fn body() -> Element {
        let key = ("default".to_owned(), InputAddress::Unbound);
        let sid = StageId(vec![StageIdSegment::Index(0)]);
        let cfg = DeadzoneConfig::new(-0.5, -0.1, 0.1, 0.5).expect("valid");
        rsx! {
            DeadzoneBody {
                mapping_key: key,
                stage_id: sid,
                config: cfg.clone(),
                root_actions: vec![Action::Deadzone { config: cfg }],
            }
        }
    }
    let html = mount_stage_body_test(body);
    assert!(
        html.contains("if-deadzone__zone--sat"),
        "saturation band class missing: {html}"
    );
    assert!(
        html.contains("if-deadzone__zone--ramp"),
        "ramp band class missing: {html}"
    );
    assert!(
        html.contains("if-deadzone__zone--dead"),
        "dead band class missing: {html}"
    );
}

#[test]
fn no_live_dot_when_input_unbound() {
    fn body() -> Element {
        let key = ("default".to_owned(), InputAddress::Unbound);
        let sid = StageId(vec![StageIdSegment::Index(0)]);
        let cfg = DeadzoneConfig::default();
        rsx! {
            DeadzoneBody {
                mapping_key: key,
                stage_id: sid,
                config: cfg.clone(),
                root_actions: vec![Action::Deadzone { config: cfg }],
            }
        }
    }
    let html = mount_stage_body_test(body);
    // The trailing `"` distinguishes the live-dot circle class from the
    // sibling `if-deadzone__live-dot-halo`. Both share the same prefix.
    assert!(
        !html.contains("if-deadzone__live-dot\""),
        "unbound input must not render a live dot: {html}"
    );
}

#[test]
fn renders_toolbar_inputs() {
    fn body() -> Element {
        let key = ("default".to_owned(), InputAddress::Unbound);
        let sid = StageId(vec![StageIdSegment::Index(0)]);
        let cfg = DeadzoneConfig::default();
        rsx! {
            DeadzoneBody {
                mapping_key: key,
                stage_id: sid,
                config: cfg.clone(),
                root_actions: vec![Action::Deadzone { config: cfg }],
            }
        }
    }
    let html = mount_stage_body_test(body);
    assert!(
        html.contains(r#"id="dz-low""#),
        "Low input id missing: {html}"
    );
    assert!(
        html.contains(r#"id="dz-cl""#),
        "CL input id missing: {html}"
    );
    assert!(
        html.contains(r#"id="dz-ch""#),
        "CH input id missing: {html}"
    );
    assert!(
        html.contains(r#"id="dz-high""#),
        "High input id missing: {html}"
    );
}
