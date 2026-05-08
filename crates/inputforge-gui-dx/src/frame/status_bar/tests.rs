//! Tests for the F7 status bar. Lock the typography deltas and the
//! surface contract in shape parallel to the right-panel passes.

#![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::status_bar::StatusBar;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn provide_minimal_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());
    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });
    use_live_capture_provider();
}

#[test]
fn status_bar_warning_badge_includes_leading_glyph() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut meta = ctx.meta;
        use_hook(move || {
            let mut snap = MetaSnapshot::default();
            snap.warnings.push("dummy".to_owned());
            meta.set(snap);
        });
        rsx! { StatusBar {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-badge--warning"),
        "warning slot must remain a Badge Warning: {html}",
    );
    assert!(
        html.contains("if-frame-status-bar__warning-glyph"),
        "warning Badge must include the leading glyph slot for visual scan parity: {html}",
    );
    assert!(
        html.contains("if-icon--sm"),
        "warning glyph must be the Phosphor warning Icon (currentColor SVG), \
         not a raw Unicode codepoint that renders as Win11 color emoji: {html}",
    );
}

#[test]
fn status_bar_device_count_numerator_uses_mono_text_class() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg = ctx.config;
        use_hook(move || {
            cfg.set(ConfigSnapshot {
                devices: vec![],
                ..ConfigSnapshot::default()
            });
        });
        rsx! { StatusBar {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-frame-status-bar__count-numerator"),
        "device count numerator must carry a mono class so the digits read as data \
         rather than chrome: {html}",
    );
}

#[test]
fn status_bar_css_locks_surface_contract() {
    let css = include_str!("../../../assets/components/status-bar.css");
    let block = css
        .split(".if-status-bar {")
        .nth(1)
        .expect(".if-status-bar rule present")
        .split('}')
        .next()
        .expect(".if-status-bar rule closed");
    assert!(
        block.contains("background: var(--color-bg-sunken);"),
        ".if-status-bar must declare bg-sunken per DESIGN.md section 7: {block}",
    );
    assert!(
        block.contains("border-top: 1px solid var(--color-border-strong);"),
        ".if-status-bar must declare a 1px strong-border top hairline per \
         DESIGN.md section 7: {block}",
    );
    assert!(
        block.contains("height: 28px;"),
        ".if-status-bar height contract is 28px (matches egui shell): {block}",
    );
}
