use dioxus::prelude::*;

use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

mod device_panel;

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

struct PanelSpec {
    caption: &'static str,
    title: &'static str,
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
            caption: "Panel · F12",
            title: "Calibration",
            body: "F12 owns content (calibration)",
            aria: "Calibration panel",
        },
        PanelSlotEnum::Devices => PanelSpec {
            caption: "Panel · F12",
            title: "Devices",
            body: "F12 owns content",
            aria: "Devices panel",
        },
        PanelSlotEnum::Profiles => PanelSpec {
            caption: "Panel · F13",
            title: "Profiles",
            body: "F13 owns content",
            aria: "Profiles panel",
        },
        PanelSlotEnum::None => unreachable!("None branch returned above"),
    };
    let body = match s {
        PanelSlotEnum::Devices if !calib => rsx! { device_panel::DevicePanel {} },
        _ => rsx! { "{spec.body}" },
    };

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        aside {
            class: "if-panel-slot",
            "aria-label": "{spec.aria}",
            header { class: "if-panel-slot__header",
                div { class: "if-panel-slot__caption", "{spec.caption}" }
                h2 { class: "if-panel-slot__title", "{spec.title}" }
            }
            div { class: "if-panel-slot__body", {body} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dioxus_ssr::render;

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

        use_context_provider(|| ViewState {
            main_surface,
            editing_mode,
            panel_slot,
            via_calibration,
            selected_mapping,
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
}
