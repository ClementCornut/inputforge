//! Component tests for `frame::mapping_list`. Each test mounts a
//! stub-context harness (mirroring `app::tests::app_root_view_with_stub_contexts`)
//! and asserts on the rendered HTML.

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::mapping_list::MappingList;
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
fn mapping_list_mounts_with_rail_class() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail"),
        "MappingList should render the .if-rail container; got: {html}",
    );
}
