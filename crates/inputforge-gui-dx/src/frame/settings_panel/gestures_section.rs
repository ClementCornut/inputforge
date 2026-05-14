//! Gesture default thresholds section.

// Rust guideline compliant 2026-05-14

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{InputSize, IntegerInput};
use crate::context::AppContext;
use crate::frame::settings_panel::field_row::SettingsFieldRow;
use crate::frame::settings_panel::section::SettingsSection;

const DOUBLE_TAP_THRESHOLD_ID: &str = "settings-double-tap-threshold";
const LONG_PRESS_THRESHOLD_ID: &str = "settings-long-press-threshold";

/// Inclusive lower bound shared with gesture threshold validation.
const THRESHOLD_MIN_MS: usize = 1;

/// Inclusive upper bound shared with gesture threshold validation.
const THRESHOLD_MAX_MS: usize = 10_000;

pub(super) fn dispatch_default_double_tap_threshold(tx: &Sender<EngineCommand>, threshold_ms: u64) {
    if let Err(e) = tx.send(EngineCommand::SetDefaultDoubleTapThreshold { threshold_ms }) {
        tracing::warn!(
            target: "gestures_section",
            %e,
            "Failed to dispatch SetDefaultDoubleTapThreshold"
        );
    }
}

pub(super) fn dispatch_default_long_press_threshold(tx: &Sender<EngineCommand>, threshold_ms: u64) {
    if let Err(e) = tx.send(EngineCommand::SetDefaultLongPressThreshold { threshold_ms }) {
        tracing::warn!(
            target: "gestures_section",
            %e,
            "Failed to dispatch SetDefaultLongPressThreshold"
        );
    }
}

fn commit_threshold_if_changed(
    tx: &Sender<EngineCommand>,
    current_threshold_ms: u64,
    candidate: usize,
    dispatch: fn(&Sender<EngineCommand>, u64),
) -> bool {
    let threshold_ms = candidate as u64;
    if threshold_ms == current_threshold_ms {
        return false;
    }
    dispatch(tx, threshold_ms);
    true
}

#[component]
pub(crate) fn GesturesSection() -> Element {
    let ctx = use_context::<AppContext>();
    let commands = ctx.commands.clone();
    let (double_tap_threshold_ms, long_press_threshold_ms) = {
        let settings = ctx.settings.read();
        (
            settings.default_double_tap_threshold_ms,
            settings.default_long_press_threshold_ms,
        )
    };

    let mut double_tap_value = use_signal(|| threshold_to_usize(double_tap_threshold_ms));
    use_effect(use_reactive!(|double_tap_threshold_ms| {
        double_tap_value.set(threshold_to_usize(double_tap_threshold_ms));
    }));

    let mut long_press_value = use_signal(|| threshold_to_usize(long_press_threshold_ms));
    use_effect(use_reactive!(|long_press_threshold_ms| {
        long_press_value.set(threshold_to_usize(long_press_threshold_ms));
    }));

    let commands_for_double_tap = commands.clone();
    let on_double_tap_commit = move |value: usize| {
        double_tap_value.set(value);
        commit_threshold_if_changed(
            &commands_for_double_tap,
            double_tap_threshold_ms,
            value,
            dispatch_default_double_tap_threshold,
        );
    };

    let commands_for_long_press = commands.clone();
    let on_long_press_commit = move |value: usize| {
        long_press_value.set(value);
        commit_threshold_if_changed(
            &commands_for_long_press,
            long_press_threshold_ms,
            value,
            dispatch_default_long_press_threshold,
        );
    };

    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Double-tap threshold".to_owned(),
                    helper: "Used when creating new Tap gesture stages.".to_owned(),
                    control_id: DOUBLE_TAP_THRESHOLD_ID.to_owned(),
                    control: rsx! {
                        IntegerInput {
                            id: Some(DOUBLE_TAP_THRESHOLD_ID.to_owned()),
                            value: double_tap_value,
                            min: THRESHOLD_MIN_MS,
                            max: THRESHOLD_MAX_MS,
                            size: InputSize::Sm,
                            oncommit: on_double_tap_commit,
                        }
                    },
                }
                SettingsFieldRow {
                    label: "Long-press threshold".to_owned(),
                    helper: "Used when creating new Press gesture stages.".to_owned(),
                    control_id: LONG_PRESS_THRESHOLD_ID.to_owned(),
                    control: rsx! {
                        IntegerInput {
                            id: Some(LONG_PRESS_THRESHOLD_ID.to_owned()),
                            value: long_press_value,
                            min: THRESHOLD_MIN_MS,
                            max: THRESHOLD_MAX_MS,
                            size: InputSize::Sm,
                            oncommit: on_long_press_commit,
                        }
                    },
                }
            },
        }
    }
}

fn threshold_to_usize(threshold_ms: u64) -> usize {
    usize::try_from(threshold_ms).unwrap_or(THRESHOLD_MAX_MS)
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::engine::EngineCommand;
    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };
    use crate::toast::{ToastQueue, ToastState};

    use super::GesturesSection;

    fn HarnessWithDefaults(double_tap_threshold_ms: u64, long_press_threshold_ms: u64) -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(|| SettingsSnapshot {
            default_double_tap_threshold_ms: double_tap_threshold_ms,
            default_long_press_threshold_ms: long_press_threshold_ms,
            ..SettingsSnapshot::default()
        });

        use_context_provider(|| AppContext {
            state,
            commands,
            settings,
            meta,
            config,
            live,
        });

        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });

        rsx! { GesturesSection {} }
    }

    fn HarnessDefaults325725() -> Element {
        HarnessWithDefaults(325, 725)
    }

    #[test]
    fn gesture_settings_render_two_threshold_rows() {
        let mut vdom = VirtualDom::new(HarnessDefaults325725);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains("Double-tap threshold"),
            "missing double-tap label: {html}"
        );
        assert!(html.contains("325"), "missing double-tap value: {html}");
        assert!(
            html.contains("Long-press threshold"),
            "missing long-press label: {html}"
        );
        assert!(html.contains("725"), "missing long-press value: {html}");
    }

    #[test]
    fn dispatch_default_double_tap_threshold_sends_command() {
        let (tx, rx) = mpsc::channel();

        super::dispatch_default_double_tap_threshold(&tx, 375);

        assert_eq!(
            rx.recv().expect("command should be sent"),
            EngineCommand::SetDefaultDoubleTapThreshold { threshold_ms: 375 }
        );
    }

    #[test]
    fn dispatch_default_long_press_threshold_sends_command() {
        let (tx, rx) = mpsc::channel();

        super::dispatch_default_long_press_threshold(&tx, 850);

        assert_eq!(
            rx.recv().expect("command should be sent"),
            EngineCommand::SetDefaultLongPressThreshold { threshold_ms: 850 }
        );
    }

    #[test]
    fn unchanged_threshold_commit_is_noop() {
        let (tx, rx) = mpsc::channel();

        assert!(!super::commit_threshold_if_changed(
            &tx,
            500,
            500,
            super::dispatch_default_double_tap_threshold,
        ));

        rx.try_recv().unwrap_err();
    }

    #[test]
    fn changed_threshold_commit_dispatches() {
        let (tx, rx) = mpsc::channel();

        assert!(super::commit_threshold_if_changed(
            &tx,
            500,
            650,
            super::dispatch_default_long_press_threshold,
        ));

        assert_eq!(
            rx.recv().expect("command should be sent"),
            EngineCommand::SetDefaultLongPressThreshold { threshold_ms: 650 }
        );
    }
}
