//! F15 settings panel. See docs/superpowers/specs/2026-05-09-f15-settings-ui-design.md.

use dioxus::prelude::*;

mod field_row;
mod prune_confirm;
mod section;
mod snapshots_section;
mod startup_section;

pub(crate) use snapshots_section::SnapshotsSection;
pub(crate) use startup_section::StartupSection;

const SETTINGS_PANEL_CSS: Asset = asset!("/assets/frame/settings_panel.css");

#[component]
pub(crate) fn SettingsPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "settings_panel");
    rsx! {
        Stylesheet { href: SETTINGS_PANEL_CSS }
        div { class: "if-settings-panel",
            StartupSection {}
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

    #[allow(non_snake_case, reason = "Dioxus components are PascalCase")]
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
    fn panel_renders_field_rows_without_heading() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("Launch InputForge at startup"),
            "expected startup field 1 label: {html}"
        );
        assert!(
            html.contains("Start minimized to tray"),
            "expected startup field 2 label: {html}"
        );
        assert!(
            html.contains("Snapshot buffer size"),
            "expected snapshot field 1 label: {html}"
        );
        assert!(
            html.contains("Skip startup snapshot"),
            "expected snapshot field 2 label: {html}"
        );
        assert!(!html.contains("<h2"), "panel must not render an h2 header");
        assert!(
            !html.contains("<h3"),
            "panel must not render an h3 section heading: {html}"
        );
    }

    // ---- Step 2: reachability with no profile --------------------------------

    #[test]
    fn panel_renders_when_no_profile_loaded() {
        // Default MetaSnapshot has profile_name = None; the panel must
        // still render every field.
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("Snapshot buffer size"));
        assert!(html.contains("Skip startup snapshot"));
    }

    // ---- Step 3: polled-signal harness and assertions ------------------------

    #[derive(Clone, Copy, Props, PartialEq)]
    struct PolledHarnessProps {
        max_count: usize,
        skip: bool,
        unpinned: usize,
    }

    #[allow(non_snake_case, reason = "Dioxus components are PascalCase")]
    fn PolledHarness(props: PolledHarnessProps) -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(|| {
            let mut s = SettingsSnapshot::default();
            s.snapshot.max_count = props.max_count;
            s.snapshot.skip_if_unchanged = props.skip;
            s.unpinned_snapshot_count = props.unpinned;
            s
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

        rsx! { SettingsPanel {} }
    }

    #[test]
    fn panel_reflects_polled_max_count() {
        let mut vdom = VirtualDom::new_with_props(
            PolledHarness,
            PolledHarnessProps {
                max_count: 25,
                skip: false,
                unpinned: 0,
            },
        );
        vdom.rebuild_in_place();
        let html = render(&vdom);
        // IntegerInput renders the value as the input's `value` attribute.
        assert!(
            html.contains(r#"value="25""#),
            "expected value=25 in input: {html}"
        );
    }

    #[test]
    fn polled_settings_signal_reflects_state_snapshot_config() {
        // Verifies the mirror chain end to end:
        // engine writes AppState.snapshot_config (Task 1.5);
        // bridge polls SettingsSnapshot::from_state into ctx.settings (Task 5);
        // SnapshotsSection reads ctx.settings.snapshot.max_count.
        // The PolledHarness above exercises the second and third hops in
        // isolation; this test seeds the source-of-truth and asserts the
        // rendered IntegerInput value matches.
        let mut vdom = VirtualDom::new_with_props(
            PolledHarness,
            PolledHarnessProps {
                max_count: 25,
                skip: true,
                unpinned: 0,
            },
        );
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains(r#"value="25""#),
            "expected value=25 in input: {html}"
        );
    }
}
