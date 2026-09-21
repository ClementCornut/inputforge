//! Engine availability and routing notices shared by every input backend.
use crate::{
    context::AppContext,
    frame::{ViewState, view_state::PanelSlot},
};
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::EngineStatus};

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus RSX event attributes produce redundant qualifications"
)]
pub(crate) fn EngineOfflineBanner() -> Element {
    let ctx = use_context::<AppContext>();
    let meta = ctx.meta.read().clone();
    let pending = ctx.controller_changes_pending();
    let view = try_use_context::<ViewState>();
    let output_failure = meta.session.output_failures.first().cloned();
    let message = if let Some(failure) = &output_failure {
        Some(super::mapping_issue_copy::output_failure_message(failure))
    } else if meta.session.offline {
        Some(meta.session.error.clone().unwrap_or_else(|| {
            "Engine offline. Reopen InputForge to restore input monitoring.".to_owned()
        }))
    } else if pending {
        Some("Apply controller changes in Devices before starting routing.".to_owned())
    } else {
        meta.session
            .error
            .clone()
            .or(meta.session.notice.clone())
            .or_else(|| {
                (meta.engine_status == EngineStatus::Stopped)
                    .then(|| "Routing stopped. Inputs remain available.".to_owned())
            })
    };
    let Some(message) = message else {
        return rsx! {};
    };
    let faulted = output_failure.is_some()
        || meta.session.error.is_some()
        || meta.engine_status == EngineStatus::Faulted;
    let running = meta.engine_status == EngineStatus::Running;
    let commands = ctx.commands.clone();
    rsx! {
        div { class: "if-editor__offline-banner", role: if faulted { "alert" } else { "status" },
            "aria-live": "polite",
            div { class: "if-editor__offline-text", "{message}" }
            if output_failure.is_some() {
                if meta.session.output_failures.iter().any(|failure| !failure.cleanup.is_empty()) {
                    div { class: "if-editor__offline-text", "Cleanup also failed. Review the technical details before retrying." }
                }
                details { summary { "Technical details" }
                    for failure in &meta.session.output_failures {
                        code { "{failure.output} {failure.phase} ({failure.category:?}): {super::mapping_issue_copy::technical_details(failure)}" }
                    }
                }
            }
            if !meta.session.offline && pending {
                button { r#type: "button", class: "if-editor__offline-action", disabled: view.is_none(),
                    onclick: move |_| { if let Some(mut view) = view { view.via_calibration.set(false); view.panel_slot.set(PanelSlot::Devices); } }, "Open Devices"
                }
            } else if !meta.session.offline {
                button { r#type: "button", class: "if-editor__offline-action",
                    disabled: !running && meta.profile_name.is_none(),
                    onclick: move |_| { let _ = commands.send(if running { EngineCommand::RefreshInput } else if faulted { EngineCommand::Retry } else { EngineCommand::Activate }); },
                    if running { "Refresh devices" } else if faulted { "Retry" } else { "Start" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
    use inputforge_core::{
        mode::Modes,
        output::{OutputFailure, OutputKind, OutputPhase},
        profile::Profile,
        state::AppState,
    };
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    #[derive(Clone, Copy, Props, PartialEq)]
    struct Props {
        offline: bool,
        pending: bool,
    }

    fn harness(props: Props) -> Element {
        let mut state = AppState::new();
        state.engine_status = EngineStatus::Running;
        state.session.offline = props.offline;
        state.session.error = Some("Controller permission denied".to_owned());
        let mut snapshot = ConfigSnapshot::default();
        if props.pending {
            use inputforge_core::{
                profile::controllers::ControllerConfig, types::VirtualDeviceConfig,
            };
            state.engine_status = EngineStatus::Stopped;
            state.session.output_active = true;
            state.session.output_layout = vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 2,
                hat_count: 0,
            }];
            snapshot.controllers = Some(ControllerConfig {
                virtual_devices: vec![VirtualDeviceConfig {
                    device_id: 1,
                    axes: vec![],
                    button_count: 3,
                    hat_count: 0,
                }],
                ..Default::default()
            });
        }
        let meta = use_signal(|| MetaSnapshot::from_state(&state));
        let config = use_signal(|| snapshot);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);
        let (commands, _) = mpsc::channel();
        use_context_provider(|| AppContext {
            state: Arc::new(RwLock::new(state)),
            commands,
            meta,
            config,
            live,
            settings,
        });
        rsx! { EngineOfflineBanner {} }
    }

    fn injection_harness() -> Element {
        let mut state = AppState::new();
        state.engine_status = EngineStatus::Faulted;
        state.active_profile = Some(Profile::new(
            "empty".into(),
            vec![],
            Modes::new(vec!["Default".into()]).unwrap(),
            vec![],
            vec![],
            "Default".into(),
        ));
        state.session.error = Some("opaque combined backend error".into());
        state.session.output_failures = vec![
            OutputFailure {
                output: OutputKind::Mouse,
                phase: OutputPhase::Readiness,
                category: std::io::ErrorKind::InvalidData,
                details: "native verification detail".into(),
                cleanup: vec!["native cleanup detail".into()],
            },
            OutputFailure {
                output: OutputKind::Keyboard,
                phase: OutputPhase::Emission,
                category: std::io::ErrorKind::BrokenPipe,
                details: "secondary native detail".into(),
                cleanup: vec![],
            },
        ];
        let meta = use_signal(|| MetaSnapshot::from_state(&state));
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);
        let (commands, _) = mpsc::channel();
        use_context_provider(|| AppContext {
            state: Arc::new(RwLock::new(state)),
            commands,
            meta,
            config,
            live,
            settings,
        });
        rsx! { EngineOfflineBanner {} }
    }

    #[test]
    fn runtime_errors_remain_visible_and_offline_threads_offer_no_dispatch() {
        for offline in [false, true] {
            let mut dom = VirtualDom::new_with_props(
                harness,
                Props {
                    offline,
                    pending: false,
                },
            );
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("Controller permission denied"));
            assert_eq!(html.contains("Refresh devices"), !offline);
            assert!(!html.contains("Outputs frozen"));
        }
    }
    #[test]
    fn pending_controller_changes_offer_devices_instead_of_retry() {
        let mut dom = VirtualDom::new_with_props(
            harness,
            Props {
                offline: false,
                pending: true,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Apply controller changes in Devices"));
        assert!(html.contains("Open Devices"));
        assert!(!html.contains(">Retry<"));
        assert!(!html.contains(">Start<"));
    }

    #[test]
    fn empty_profile_prefers_typed_output_guidance_with_retry_and_cleanup_details() {
        let mut dom = VirtualDom::new(injection_harness);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Mouse output could not be verified"));
        assert!(html.contains("Cleanup also failed"));
        assert!(html.contains("Technical details"));
        assert!(html.contains("native verification detail"));
        assert!(html.contains("native cleanup detail"));
        assert!(html.contains("secondary native detail"));
        assert!(!html.contains("opaque combined backend error"));
        assert!(html.contains(">Retry<"));
    }
}
