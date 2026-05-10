//! Startup preferences section (F16). Two independent switches above
//! `SnapshotsSection`. Follows the polled-into-local-Signal pattern from
//! `snapshots_section.rs:67-79` and `:124-136` to avoid the double-click
//! race within a single polling tick.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::Switch;
use crate::context::AppContext;
use crate::frame::settings_panel::field_row::SettingsFieldRow;
use crate::frame::settings_panel::section::SettingsSection;

const LAUNCH_AT_STARTUP_ID: &str = "if-settings-startup-launch-at-startup";
const START_MINIMIZED_ID: &str = "if-settings-startup-start-minimized";

/// Dispatches `SetAutostart` to the engine. Extracted so tests can call the
/// helper directly with a real `mpsc::channel` and verify the right
/// `EngineCommand` variant was sent. Returning unit (errors are logged) keeps
/// the call-site closure body trivial.
pub(super) fn dispatch_set_autostart(tx: &Sender<EngineCommand>, enabled: bool) {
    if let Err(e) = tx.send(EngineCommand::SetAutostart { enabled }) {
        tracing::warn!(target: "startup_section", %e, "Failed to dispatch SetAutostart");
    }
}

/// Dispatches `SetStartMinimizedToTray` to the engine. See
/// `dispatch_set_autostart` for the rationale behind the helper split.
pub(super) fn dispatch_set_start_minimized_to_tray(tx: &Sender<EngineCommand>, enabled: bool) {
    if let Err(e) = tx.send(EngineCommand::SetStartMinimizedToTray { enabled }) {
        tracing::warn!(target: "startup_section", %e, "Failed to dispatch SetStartMinimizedToTray");
    }
}

#[component]
pub(crate) fn StartupSection() -> Element {
    let ctx = use_context::<AppContext>();
    let settings = ctx.settings;
    let commands = ctx.commands.clone();

    let polled = settings.read().startup.clone();
    let polled_launch = polled.launch_at_startup;
    let polled_start_min = polled.start_minimized_to_tray;

    let mut launch_local = use_signal(|| polled_launch);
    use_effect(use_reactive!(|polled_launch| {
        launch_local.set(polled_launch);
    }));

    let mut start_min_local = use_signal(|| polled_start_min);
    use_effect(use_reactive!(|polled_start_min| {
        start_min_local.set(polled_start_min);
    }));

    let commands_for_launch = commands.clone();
    let on_launch_change = move |_evt: FormEvent| {
        let new_value = !launch_local();
        launch_local.set(new_value);
        dispatch_set_autostart(&commands_for_launch, new_value);
    };

    let commands_for_start_min = commands.clone();
    let on_start_min_change = move |_evt: FormEvent| {
        let new_value = !start_min_local();
        start_min_local.set(new_value);
        dispatch_set_start_minimized_to_tray(&commands_for_start_min, new_value);
    };

    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Launch InputForge at startup".to_owned(),
                    helper: "Run automatically after sign-in.".to_owned(),
                    control_id: LAUNCH_AT_STARTUP_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(LAUNCH_AT_STARTUP_ID.to_owned()),
                            checked: launch_local,
                            onchange: on_launch_change,
                        }
                    },
                }
                SettingsFieldRow {
                    label: "Start minimized to tray".to_owned(),
                    helper: "Open without showing the main window. Use the tray icon to bring it back.".to_owned(),
                    control_id: START_MINIMIZED_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(START_MINIMIZED_ID.to_owned()),
                            checked: start_min_local,
                            onchange: on_start_min_change,
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

    use inputforge_core::settings::StartupSettings;
    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };
    use crate::toast::{ToastQueue, ToastState};

    use super::StartupSection;

    fn HarnessWithStartup(launch: bool, start_min: bool) -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(|| SettingsSnapshot {
            startup: StartupSettings {
                launch_at_startup: launch,
                start_minimized_to_tray: start_min,
            },
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

        rsx! { StartupSection {} }
    }

    fn HarnessBothOn() -> Element {
        HarnessWithStartup(true, true)
    }

    fn HarnessBothOff() -> Element {
        HarnessWithStartup(false, false)
    }

    fn HarnessLaunchOnly() -> Element {
        HarnessWithStartup(true, false)
    }

    #[test]
    fn renders_two_switches_with_persisted_state() {
        let mut vdom = VirtualDom::new(HarnessBothOn);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        // Both labels are present.
        assert!(
            html.contains("Launch InputForge at startup"),
            "missing launch-at-startup label: {html}"
        );
        assert!(
            html.contains("Start minimized to tray"),
            "missing start-minimized label: {html}"
        );

        // Both inputs render with id.
        assert!(
            html.contains(r#"id="if-settings-startup-launch-at-startup""#),
            "missing launch input id"
        );
        assert!(
            html.contains(r#"id="if-settings-startup-start-minimized""#),
            "missing start-minimized input id"
        );
        assert!(
            html.matches(r#"type="checkbox""#).count() >= 2,
            "expected at least two checkbox inputs"
        );

        // dioxus-ssr 0.7.6 renders boolean HTML attributes as bare names
        // (e.g. `<input ... checked>`), never as `checked="true"`. Each
        // attribute name in the serialized output is preceded by whitespace,
        // so matching on the substring " checked" with a leading space
        // disambiguates the bare attribute from any other occurrence.
        assert_eq!(
            html.matches(" checked").count(),
            2,
            "expected 2 bare `checked` attrs in both-on render: {html}"
        );
    }

    #[test]
    fn off_state_does_not_render_checked() {
        let mut vdom = VirtualDom::new(HarnessBothOff);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        // No bare `checked` attribute when both switches are off.
        assert!(
            !html.contains(" checked"),
            "expected zero `checked` attrs when both startup fields are off: {html}"
        );
    }

    #[test]
    fn launch_only_renders_one_checked() {
        // Confirms each checkbox is bound to its own prop: launch=true,
        // start_min=false should produce exactly one bare `checked`.
        let mut vdom = VirtualDom::new(HarnessLaunchOnly);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert_eq!(
            html.matches(" checked").count(),
            1,
            "expected exactly 1 bare `checked` for launch-only render: {html}"
        );
    }
}
