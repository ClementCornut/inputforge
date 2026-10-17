//! Startup settings render app-prepared switches.
use super::{field_row::SettingsFieldRow, section::SettingsSection};
use crate::{components::Switch, settings::Intent};
use dioxus::prelude::*;
const LAUNCH_AT_STARTUP_ID: &str = "if-settings-startup-launch-at-startup";
const START_MINIMIZED_ID: &str = "if-settings-startup-start-minimized";
#[component]
pub(crate) fn StartupSection() -> Element {
    let c = super::context();
    let launch = use_memo(move || c.view.read().values.startup.launch_at_startup);
    let minimized = use_memo(move || c.view.read().values.startup.start_minimized_to_tray);
    let on_launch_change = move |_| {
        let _ = c.send(Intent::Autostart);
    };
    let on_start_min_change = move |_| {
        let _ = c.send(Intent::Minimized);
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
                            checked: launch,
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
                            checked: minimized,
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

        inputforge_app::notifications::use_provider();

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
