//! Cursor menus wait for their native mount before measuring browser layout.
use crate::components::{AnchoredMenu, MenuAnchor, MenuItem, test_events};
use dioxus::prelude::*;
use std::{
    cell::Cell,
    rc::Rc,
    task::{Context, Poll},
};

#[derive(Default)]
struct MountedDocument {
    owner: Owner<UnsyncStorage>,
    mounted: Cell<bool>,
    measurements: Cell<usize>,
}
struct Measurement(bool);
impl document::Evaluator for Measurement {
    fn send(&self, _: serde_json::Value) -> Result<(), document::EvalError> {
        Ok(())
    }
    fn poll_recv(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        Poll::Ready(Ok(if self.0 {
            serde_json::json!([240.0, 320.0, 800.0, 600.0])
        } else {
            serde_json::json!([0.0, 0.0, 800.0, 600.0])
        }))
    }
    fn poll_join(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        Poll::Pending
    }
}
impl document::Document for MountedDocument {
    fn eval(&self, js: String) -> document::Eval {
        if js.contains("window.innerWidth, window.innerHeight") {
            self.measurements.set(self.measurements.get() + 1);
            document::Eval::new(self.owner.insert(Box::new(Measurement(self.mounted.get()))))
        } else {
            document::Document::eval(&document::NoOpDocument, js)
        }
    }
}
fn fixture() -> Element {
    inputforge_app::ui::use_provider();
    rsx! {
        AnchoredMenu {
            open: Some(MenuAnchor { x: 790.0, y: 590.0 }),
            on_close: move |_| true,
            MenuItem { "Rename" }
        }
    }
}
fn settle(dom: &mut VirtualDom) {
    for _ in 0..4 {
        dom.render_immediate_to_vec();
    }
}
#[test]
fn regression_cursor_menu_waits_for_native_mount_and_then_becomes_visible() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let document = Rc::new(MountedDocument::default());
    let mut dom = VirtualDom::new(fixture);
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    settle(&mut dom);
    assert_eq!(
        document.measurements.get(),
        0,
        "the renderer has not mounted the menu yet"
    );
    assert!(dioxus_ssr::render(&dom).contains("visibility: hidden"));
    document.mounted.set(true);
    test_events::event(&dom, "mounted", 0, MountedData::new(()));
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    assert_eq!(document.measurements.get(), 1);
    assert!(!html.contains("visibility: hidden"), "{html}");
    assert!(
        html.contains("visibility: visible"),
        "Dioxus retains omitted CSS properties; the ready menu must explicitly restore visibility: {html}"
    );
    assert!(html.contains("left: 556px; top: 276px"), "{html}");
}
