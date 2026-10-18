//! Reusable controls exercise app-owned behavior through real renderer handlers.
#[path = "controls/mod.rs"]
mod fixture;
use dioxus::prelude::*;
use fixture::{Fixture, event, form, key};
#[test]
fn rapid_text_events_keep_exact_drafts_without_echoing_values_to_the_webview() {
    let mut f = Fixture::new();
    for raw in ["P", "Pr", "Profile é", "Profile été 🕹"] {
        form(&f.dom, 2, raw);
    }
    assert_eq!(&*f.values().text.peek(), "Profile été 🕹");
    let mutations = f.dom.render_immediate_to_vec();
    assert!(
        !mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::core::Mutation::SetAttribute { name: "value", .. }
        )),
        "an older native render must never replace a newer browser draft: {:?}",
        mutations.edits
    );
    assert_eq!(
        f.log.borrow().text_inputs,
        ["P", "Pr", "Profile é", "Profile été 🕹"]
    );
}
#[test]
fn numeric_typing_does_not_echo_partial_raw_drafts_to_the_webview() {
    let mut f = Fixture::new();
    for (index, values) in [(0, ["0", "0.", "0.37"]), (1, ["1", "10", "101"])] {
        for raw in values {
            form(&f.dom, index, raw);
        }
    }
    let mutations = f.dom.render_immediate_to_vec();
    assert!(
        !mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::core::Mutation::SetAttribute { name: "value", .. }
        )),
        "native numeric renders must retain the browser's newer draft: {:?}",
        mutations.edits
    );
}
#[test]
fn delayed_text_acknowledgements_do_not_reset_typing_but_external_replacements_do() {
    let mut f = Fixture::new();
    f.values().acknowledge_text.set(false);
    f.render();
    for raw in ["P", "Pr", "Profile été 🕹"] {
        form(&f.dom, 2, raw);
    }
    f.render();
    for acknowledged in ["P", "Pr", "Profile été 🕹"] {
        f.values().text.set(acknowledged.into());
        f.render();
        assert!(
            f.text_resets().is_empty(),
            "an acknowledgement overwrote the draft"
        );
    }
    f.values().text.set("New \"context\" 🕹".into());
    f.render();
    assert_eq!(f.text_resets().len(), 1);
    assert!(f.text_resets()[0].contains(r#"el.value = "New \"context\" 🕹""#));
    f.values().text.set(String::new());
    f.render();
    assert_eq!(f.text_resets().len(), 2);
    assert!(f.text_resets()[1].contains("el.value = \"\""));
}
#[test]
fn repeated_ordered_text_acknowledgements_do_not_rewind_the_latest_draft() {
    let mut f = Fixture::new();
    f.values().acknowledge_text.set(false);
    f.render();
    for raw in ["a", "ab", "a"] {
        form(&f.dom, 2, raw);
    }
    for acknowledged in ["a", "ab", "a"] {
        f.values().text.set(acknowledged.into());
        f.render();
    }
    assert!(
        f.text_resets().is_empty(),
        "an old acknowledgement replaced the current draft"
    );
}
#[test]
fn text_reset_before_the_next_render_restores_the_original_value() {
    let mut f = Fixture::new();
    form(&f.dom, 2, "Discard this draft");
    f.values().text.set(String::new());
    f.render();
    assert_eq!(f.text_resets().len(), 1);
    assert!(f.text_resets()[0].contains("el.value = \"\""));
}
#[test]
fn switching_text_context_resets_the_draft_and_ignores_old_source_updates() {
    let mut f = Fixture::new();
    f.values().acknowledge_text.set(false);
    f.render();
    form(&f.dom, 2, "Belongs to the old profile");
    f.values().other_text_context.set(true);
    f.render();
    f.render();
    assert_eq!(
        f.text_resets().len(),
        1,
        "{:?} {}",
        f.text_resets(),
        f.html()
    );
    assert!(f.text_resets()[0].contains("el.value = \"Next profile\""));
    f.values().text.set("Late old profile name".into());
    f.render();
    assert_eq!(
        f.text_resets().len(),
        1,
        "obsolete source reset the new field"
    );
}
#[test]
fn number_rejects_nonfinite_and_dispatches_enter_then_blur_once() {
    let mut f = Fixture::new();
    for raw in ["NaN", "inf", "", "broken"] {
        form(&f.dom, 0, raw);
        event(&f.dom, "blur", 0, SerializedFocusData {});
    }
    assert_eq!(f.log.borrow().numbers, [] as [f64; 0]);
    form(&f.dom, 0, "2.5");
    key(&f.dom, 0, Key::Enter);
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [1.0]);
    f.render();
    assert!(
        f.html().contains("value=\"2.5\""),
        "failed delivery must retain raw draft"
    );
}
#[test]
fn number_external_updates_preserve_typing_and_escape_restores_latest_value() {
    let mut f = Fixture::new();
    form(&f.dom, 0, "0.37");
    f.values().number.set(0.8);
    f.render();
    assert!(f.html().contains("value=\"0.37\""));
    key(&f.dom, 0, Key::Escape);
    event(&f.dom, "blur", 0, SerializedFocusData {});
    f.render();
    assert_eq!(f.log.borrow().numbers, [] as [f64; 0]);
    assert!(f.html().contains("value=\"0.80\""));
    f.values().number.set(0.6);
    f.render();
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert!(
        f.log.borrow().numbers.is_empty(),
        "unmodified blur must not restore old values"
    );
}
#[test]
fn integer_invalid_boundaries_and_escape_do_not_duplicate_dispatch() {
    let mut f = Fixture::new();
    for raw in ["", "-1", "3.14", "NaN", "11"] {
        form(&f.dom, 1, raw);
        key(&f.dom, 1, Key::Enter);
        event(&f.dom, "blur", 1, SerializedFocusData {});
    }
    assert_eq!(f.log.borrow().invalid.len(), 5);
    form(&f.dom, 1, " 10 ");
    key(&f.dom, 1, Key::Enter);
    event(&f.dom, "blur", 1, SerializedFocusData {});
    assert_eq!(f.log.borrow().integers, [10]);
    form(&f.dom, 1, "4");
    f.values().integer.set(7);
    f.render();
    key(&f.dom, 1, Key::Escape);
    event(&f.dom, "blur", 1, SerializedFocusData {});
    f.render();
    assert_eq!(f.log.borrow().integers, [10]);
    assert!(f.html().contains("value=\"7\""));
}
#[test]
fn disabled_controls_and_boundary_steps_ignore_synthetic_activation() {
    let mut f = Fixture::new();
    f.values().number.set(1.0);
    f.render();
    event(&f.dom, "click", 0, SerializedMouseData::default());
    event(&f.dom, "click", 1, SerializedMouseData::default());
    assert_eq!(f.log.borrow().steps, [1.0, 0.75]);
    f.values().disabled.set(true);
    f.render();
    form(&f.dom, 0, "0.5");
    event(&f.dom, "blur", 0, SerializedFocusData {});
    form(&f.dom, 1, "3");
    event(&f.dom, "blur", 1, SerializedFocusData {});
    event(&f.dom, "click", 0, SerializedMouseData::default());
    event(&f.dom, "click", 2, SerializedMouseData::default());
    assert_eq!(f.log.borrow().numbers, [] as [f64; 0]);
    assert_eq!(f.log.borrow().integers, [] as [usize; 0]);
    assert_eq!(f.log.borrow().steps.len(), 2);
    assert_eq!(f.log.borrow().segments, 0);
}
#[test]
fn text_navigation_isolated_but_commit_and_cancel_bubble_with_accessible_links() {
    let f = Fixture::new();
    for k in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
        Key::Home,
        Key::End,
    ] {
        let event = key(&f.dom, 2, k);
        assert!(!event.propagates());
    }
    for k in [Key::Enter, Key::Escape] {
        assert!(key(&f.dom, 2, k).propagates());
    }
    assert_eq!(f.log.borrow().text_keys, 8);
    let html = f.html();
    assert!(html.contains("aria-describedby=\"text-error\""));
    assert!(html.contains("for=\"number\""));
    assert!(html.contains("aria-label=\"Control tabs\""));
}
#[test]
fn tabs_wrap_skip_disabled_and_observe_removed_items() {
    let mut f = Fixture::new();
    key(&f.dom, 3, Key::ArrowRight);
    assert_eq!(f.log.borrow().tabs, ["c"]);
    f.render();
    key(&f.dom, 3, Key::ArrowRight);
    assert_eq!(f.log.borrow().tabs, ["c", "a"]);
    key(&f.dom, 3, Key::End);
    assert_eq!(f.log.borrow().tabs.last().unwrap(), "c");
    f.values().show_c.set(false);
    f.render();
    key(&f.dom, 3, Key::ArrowLeft);
    assert_eq!(f.log.borrow().tabs.last().unwrap(), "a");
    f.values().disabled.set(true);
    f.render();
    let count = f.log.borrow().tabs.len();
    key(&f.dom, 3, Key::Home);
    event(&f.dom, "click", 3, SerializedMouseData::default());
    assert_eq!(f.log.borrow().tabs.len(), count);
}
#[test]
fn failed_numeric_delivery_can_retry_after_refocus_without_retyping() {
    let f = Fixture::new();
    form(&f.dom, 0, "0.7");
    key(&f.dom, 0, Key::Enter);
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [0.7]);
    event(&f.dom, "focus", 0, SerializedFocusData {});
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [0.7, 0.7]);
}
#[test]
fn typing_after_enter_before_blur_commits_the_new_edit() {
    let f = Fixture::new();
    form(&f.dom, 0, "0.2");
    key(&f.dom, 0, Key::Enter);
    form(&f.dom, 0, "0.3");
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [0.2, 0.3]);
}
#[test]
fn committing_current_number_does_not_block_external_updates_or_recommit_stale_text() {
    let mut f = Fixture::new();
    form(&f.dom, 0, "0.5");
    key(&f.dom, 0, Key::Enter);
    event(&f.dom, "blur", 0, SerializedFocusData {});
    f.render();
    f.values().number.set(0.8);
    f.render();
    assert!(
        f.html().contains("value=\"0.80\""),
        "same-value commit must acknowledge"
    );
    event(&f.dom, "focus", 0, SerializedFocusData {});
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [0.5]);
}
#[test]
fn committing_current_integer_does_not_block_external_updates_or_recommit_stale_text() {
    let mut f = Fixture::new();
    form(&f.dom, 1, "2");
    key(&f.dom, 1, Key::Enter);
    event(&f.dom, "blur", 1, SerializedFocusData {});
    f.render();
    f.values().integer.set(7);
    f.render();
    assert!(
        f.html().contains("value=\"7\""),
        "same-value commit must acknowledge"
    );
    event(&f.dom, "focus", 1, SerializedFocusData {});
    event(&f.dom, "blur", 1, SerializedFocusData {});
    assert_eq!(f.log.borrow().integers, [2]);
}
#[test]
fn step_only_number_acknowledges_current_text_without_an_oncommit_consumer() {
    let mut f = Fixture::new();
    f.values().commit_enabled.set(false);
    f.render();
    form(&f.dom, 0, "0.5");
    key(&f.dom, 0, Key::Enter);
    event(&f.dom, "blur", 0, SerializedFocusData {});
    event(&f.dom, "click", 0, SerializedMouseData::default());
    f.render();
    assert_eq!(f.log.borrow().numbers, [] as [f64; 0]);
    assert_eq!(f.log.borrow().steps, [0.75]);
    assert!(
        f.html().contains("value=\"0.75\""),
        "step-only owner must show its published value"
    );
}
#[test]
fn integer_without_oncommit_acknowledges_current_text_before_external_updates() {
    let mut f = Fixture::new();
    f.values().commit_enabled.set(false);
    f.render();
    form(&f.dom, 1, "2");
    key(&f.dom, 1, Key::Enter);
    event(&f.dom, "blur", 1, SerializedFocusData {});
    f.values().integer.set(7);
    f.render();
    assert_eq!(f.log.borrow().integers, [] as [usize; 0]);
    assert!(f.html().contains("value=\"7\""));
}
#[test]
fn successful_step_replaces_an_older_typed_draft_after_publication() {
    let mut f = Fixture::new();
    f.values().commit_enabled.set(false);
    f.render();
    form(&f.dom, 0, "0.37");
    event(&f.dom, "click", 0, SerializedMouseData::default());
    f.render();
    assert_eq!(f.log.borrow().steps, [0.75]);
    assert!(f.html().contains("value=\"0.75\""));
}
#[test]
fn failed_step_retains_an_older_typed_draft() {
    let mut f = Fixture::new();
    form(&f.dom, 0, "0.37");
    event(&f.dom, "click", 0, SerializedMouseData::default());
    f.render();
    assert_eq!(f.log.borrow().steps, [0.75]);
    assert!(f.html().contains("value=\"0.37\""));
}
#[test]
fn fresh_typing_after_a_step_survives_that_steps_publication() {
    let mut f = Fixture::new();
    f.values().commit_enabled.set(false);
    f.render();
    event(&f.dom, "click", 0, SerializedMouseData::default());
    form(&f.dom, 0, "0.37");
    f.render();
    assert!(f.html().contains("value=\"0.37\""));
}
#[test]
fn failed_step_at_the_bound_preserves_a_different_typed_draft() {
    let mut f = Fixture::new();
    f.values().number.set(1.0);
    f.render();
    form(&f.dom, 0, "0.37");
    event(&f.dom, "click", 0, SerializedMouseData::default());
    f.render();
    assert_eq!(f.log.borrow().steps, [1.0]);
    assert!(f.html().contains("value=\"0.37\""));
}
#[test]
fn later_failed_blur_commit_keeps_its_draft_when_an_older_step_publishes() {
    let mut f = Fixture::new();
    form(&f.dom, 0, "0.37");
    event(&f.dom, "click", 0, SerializedMouseData::default());
    event(&f.dom, "focus", 0, SerializedFocusData {});
    event(&f.dom, "blur", 0, SerializedFocusData {});
    assert_eq!(f.log.borrow().numbers, [0.37]);
    f.values().number.set(0.75);
    f.render();
    assert!(
        f.html().contains("value=\"0.37\""),
        "older step must not erase the failed newer edit"
    );
}
