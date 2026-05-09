//! F15 settings panel. See docs/superpowers/specs/2026-05-09-f15-settings-ui-design.md.

use dioxus::prelude::*;

mod field_row;
mod prune_confirm;
mod section;
mod snapshots_section;

pub(crate) use snapshots_section::SnapshotsSection;

const SETTINGS_PANEL_CSS: Asset = asset!("/assets/frame/settings_panel.css");

#[component]
pub(crate) fn SettingsPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "settings_panel");
    rsx! {
        Stylesheet { href: SETTINGS_PANEL_CSS }
        div { class: "if-settings-panel",
            SnapshotsSection {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };
    use crate::toast::{ToastQueue, ToastState};

    use super::SettingsPanel;

    #[allow(non_snake_case)]
    fn Harness() -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);

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

        rsx! { SettingsPanel {} }
    }

    #[test]
    fn panel_renders_snapshots_section_heading() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("Snapshots"),
            "expected section heading: {html}"
        );
        assert!(
            html.contains("Snapshot buffer size"),
            "expected field 1 label: {html}"
        );
        assert!(
            html.contains("Skip startup snapshot"),
            "expected field 2 label: {html}"
        );
        assert!(!html.contains("<h2"), "panel must not render an h2 header");
    }
}
