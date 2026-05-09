use dioxus::prelude::*;

use crate::frame::profiles::ProfilesPanel;
use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

mod device_panel;

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

struct PanelSpec {
    body: &'static str,
    aria: &'static str,
}

#[component]
pub(crate) fn PanelSlot() -> Element {
    tracing::trace!(target: "frame::render", region = "panel_slot");
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());
    let via_calib = use_memo(move || *view.via_calibration.read());

    let s = *slot.read();
    if matches!(s, PanelSlotEnum::None) {
        return rsx! { Stylesheet { href: PANEL_SLOT_CSS } };
    }

    let calib = *via_calib.read();
    let spec = match s {
        PanelSlotEnum::Devices if calib => PanelSpec {
            body: "F12 owns content (calibration)",
            aria: "Calibration panel",
        },
        PanelSlotEnum::Devices => PanelSpec {
            body: "F12 owns content",
            aria: "Devices panel",
        },
        PanelSlotEnum::Profiles => PanelSpec {
            body: "",
            aria: "Profiles panel",
        },
        PanelSlotEnum::None => unreachable!("None branch returned above"),
    };
    let body = match s {
        PanelSlotEnum::Devices if !calib => rsx! { device_panel::DevicePanel {} },
        PanelSlotEnum::Profiles => rsx! { ProfilesPanel {} },
        _ => rsx! { div { class: "if-panel-slot__placeholder", "{spec.body}" } },
    };

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        aside {
            class: "if-panel-slot",
            "aria-label": "{spec.aria}",
            div { class: "if-panel-slot__body", {body} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, mpsc};

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };
    use dioxus_ssr::render;
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;

    #[derive(Clone, Copy, Props, PartialEq)]
    struct TestHarnessProps {
        slot: PanelSlotEnum,
        #[props(default)]
        via_calibration: bool,
    }

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn TestHarness(props: TestHarnessProps) -> Element {
        let main_surface = use_signal(Default::default);
        let editing_mode = use_signal(|| "Default".to_owned());
        let panel_slot = use_signal(|| props.slot);
        let via_calibration = use_signal(|| props.via_calibration);
        let selected_mapping = use_signal(|| None);
        let profiles_panel = use_signal(crate::frame::view_state::ProfilesPanelState::default);
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let settings = use_signal(SettingsSnapshot::default);
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);

        use_context_provider(|| ViewState {
            main_surface,
            editing_mode,
            panel_slot,
            via_calibration,
            selected_mapping,
            profiles_panel,
        });
        use_context_provider(|| AppContext {
            state,
            commands,
            settings,
            meta,
            config,
            live,
        });

        rsx! { PanelSlot {} }
    }

    fn render_slot(slot: PanelSlotEnum) -> String {
        let mut vdom = VirtualDom::new_with_props(
            TestHarness,
            TestHarnessProps {
                slot,
                via_calibration: false,
            },
        );
        vdom.rebuild_in_place();
        render(&vdom)
    }

    #[test]
    fn devices_and_profiles_share_stable_aside_shell() {
        for slot in [PanelSlotEnum::Devices, PanelSlotEnum::Profiles] {
            let html = render_slot(slot);

            assert!(
                html.contains(r#"<aside class="if-panel-slot""#),
                "slot {slot:?} did not render the stable panel shell: {html}"
            );
        }
    }

    #[test]
    fn panel_header_omits_placeholder_caption() {
        let html = render_slot(PanelSlotEnum::Devices);

        assert!(!html.contains("Panel"));
        assert!(!html.contains("F12"));
        assert!(!html.contains("if-panel-slot__caption"));
        assert!(!html.contains("if-panel-slot__header"));
        assert!(!html.contains("if-panel-slot__title"));
        assert!(!html.contains("<h2"));
        assert!(!html.contains(">Devices<"));
    }
}
