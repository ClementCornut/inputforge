//! Validation failures stay attached to their mapping while valid mappings run.
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    context::AppContext,
    frame::{MappingKey, ViewState, view_state::PanelSlot},
};
use dioxus::prelude::*;
use inputforge_core::state::{InputIssueKind, MappingIssueReason, OutputIssueKind};

#[component]
pub(super) fn MappingIssues(mapping_key: MappingKey) -> Element {
    let ctx = use_context::<AppContext>();
    let view = try_use_context::<ViewState>();
    let meta = ctx.meta.read();
    let config = ctx.config.read();
    rsx! {
        for issue in meta.session.mapping_issues.iter().filter(|issue| issue.mode == mapping_key.0 && issue.input == mapping_key.1) {
            div { class: "if-editor__offline-banner", role: "alert",
                div { class: "if-editor__offline-text",
                    p { "{super::mapping_issue_copy::message(&issue.reason, &config)}" }
                    if show_devices(&issue.reason) {
                        if let Some(mut view) = view {
                            Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary,
                                onclick: move |_| { view.via_calibration.set(false); view.panel_slot.set(PanelSlot::Devices); }, "Open Devices"
                            }
                        }
                    }
                    details { summary { "Technical details" } code { "{issue.reason:?}" } }
                }
            }
        }
    }
}

fn show_devices(reason: &MappingIssueReason) -> bool {
    matches!(
        reason,
        MappingIssueReason::Input {
            problem: InputIssueKind::Unselected
                | InputIssueKind::Disconnected
                | InputIssueKind::NotReady,
            ..
        } | MappingIssueReason::Output {
            problem: OutputIssueKind::MissingController | OutputIssueKind::MissingControl,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
    use inputforge_core::{
        state::{AppState, MappingIssue},
        types::{DeviceId, InputAddress, InputId},
    };
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    fn harness() -> Element {
        let input = InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mut state = AppState::new();
        state.session.mapping_issues = vec![
            MappingIssue {
                input: input.clone(),
                mode: "Default".to_owned(),
                reason: MappingIssueReason::KeyboardUnavailable,
            },
            MappingIssue {
                input: input.clone(),
                mode: "Other".to_owned(),
                reason: MappingIssueReason::MissingMode {
                    mode: "Other mode issue".to_owned(),
                },
            },
            MappingIssue {
                input: InputAddress::Unbound,
                mode: "Default".to_owned(),
                reason: MappingIssueReason::MissingMode {
                    mode: "Other input issue".to_owned(),
                },
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
        rsx! { MappingIssues { mapping_key: ("Default".to_owned(), input) } }
    }

    #[test]
    fn editor_reports_only_the_selected_mapping_issue() {
        let mut dom = VirtualDom::new(harness);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Keyboard output isn’t available here. Choose another output."));
        assert!(!html.contains("Mapping disabled:"));
        assert!(!html.contains("Other mode issue"));
        assert!(!html.contains("Other input issue"));
    }
}
