//! Gesture defaults render prepared values and forward commits.
use super::{field_row::SettingsFieldRow, section::SettingsSection};
use crate::{
    components::{InputSize, IntegerInput},
    settings::Intent,
};
use dioxus::prelude::*;
const DOUBLE_TAP_THRESHOLD_ID: &str = "settings-double-tap-threshold";
const LONG_PRESS_THRESHOLD_ID: &str = "settings-long-press-threshold";
#[component]
pub(crate) fn GesturesSection() -> Element {
    let c = super::context();
    let double_tap_value = use_memo(move || {
        usize::try_from(c.view.read().values.default_double_tap_threshold_ms).unwrap_or(10_000)
    });
    let long_press_value = use_memo(move || {
        usize::try_from(c.view.read().values.default_long_press_threshold_ms).unwrap_or(10_000)
    });
    let on_double_tap_commit = move |v| {
        let _ = c.send(Intent::DoubleTap(v));
    };
    let on_long_press_commit = move |v| {
        let _ = c.send(Intent::LongPress(v));
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
                            min: 1,
                            max: 10_000,
                            size: InputSize::Sm,
                            oncommit: on_double_tap_commit,
                            oninvalid: move |error| { let _ = c.send(Intent::InvalidGesture(error)); },
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
                            min: 1,
                            max: 10_000,
                            size: InputSize::Sm,
                            oncommit: on_long_press_commit,
                            oninvalid: move |error| { let _ = c.send(Intent::InvalidGesture(error)); },
                        }
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };

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

        inputforge_app::notifications::use_provider();

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
}
