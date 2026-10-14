mod apply;
mod controls;
mod support;
use crate::frame::view_state::MainSurface;
use dioxus::prelude::*;
use support::{click, mount};
#[test]
fn failed_batch_delivery_retains_wizard_and_has_visible_error_without_success() {
    let (mut dom, f, rx) = mount();
    drop(rx);
    click(&dom, "Apply 13 mappings");
    dom.render_immediate_to_vec();
    assert_eq!(
        *f.view.borrow().unwrap().main_surface.peek(),
        MainSurface::BulkMap
    );
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("role=\"alert\""),
        "missing visible delivery error: {html}"
    );
    assert!(!format!("{:?}", f.toasts.borrow().unwrap().peek()).contains("Created"));
}
#[test]
fn stale_batch_apply_rejects_disconnected_source_before_rerender() {
    let (mut dom, f, rx) = mount();
    f.state.write().devices[0].connected = false;
    click(&dom, "Apply 13 mappings");
    dom.render_immediate_to_vec();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(
        *f.view.borrow().unwrap().main_surface.peek(),
        MainSurface::BulkMap
    );
}
