//! Portal cleanup and reopening regressions.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::core::{AttributeValue, Mutation, Mutations};
use dioxus::prelude::*;
use dioxus_ssr::render;
use inputforge_gui_dx::components::Portal;

#[derive(Default)]
struct RecordingDocument(RefCell<Vec<String>>);

impl document::Document for RecordingDocument {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}

#[test]
fn portal_cleanup_preserves_the_reopening_anchor() {
    fn removable_portal() -> Element {
        let visible = use_context::<Rc<Cell<bool>>>();
        rsx! {
            if visible.get() {
                Portal { "content" }
            }
        }
    }

    let visible = Rc::new(Cell::new(true));
    let document = Rc::new(RecordingDocument::default());
    let mut vdom = VirtualDom::new(removable_portal);
    vdom.provide_root_context(Rc::clone(&visible));
    vdom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    let mut mounted = vdom.rebuild_to_vec();
    for _ in 0..3 {
        vdom.render_immediate(&mut dioxus::core::NoOpMutations);
        let html = render(&vdom);
        assert!(html.contains("class=\"if-portal\""));
        let (attribute_index, moved_node, portal_id) = mounted
            .edits
            .iter()
            .enumerate()
            .find_map(|(index, mutation)| match mutation {
                Mutation::SetAttribute {
                    name: "id",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } if value.starts_with("if-portal-") => Some((index, *id, value)),
                _ => None,
            })
            .expect("portal must assign an id to its moved element");
        let anchor = mounted.edits[..attribute_index]
            .iter()
            .rev()
            .find_map(|mutation| match mutation {
                Mutation::LoadTemplate { id, .. } => Some(*id),
                _ => None,
            })
            .expect("portal must load its root element");
        let selector = format!("document.getElementById('{portal_id}')");
        assert!(document.0.borrow().iter().any(|script| {
            script.contains(&selector) && script.contains("root.appendChild(el)")
        }));
        document.0.borrow_mut().clear();

        visible.set(false);
        vdom.mark_dirty(ScopeId::APP);
        let closed = vdom.render_immediate_to_vec();
        assert!(!render(&vdom).contains("class=\"if-portal\""));
        assert!(
            document.0.borrow().iter().any(|script| {
                script.contains(&selector) && script.contains("if (el) el.remove();")
            }),
            "unmount must dispatch cleanup before returning"
        );
        assert!(
            closed.edits.iter().any(|mutation| matches!(
                mutation, Mutation::ReplaceWith { id, .. } if *id == anchor
            )),
            "closing must replace the root at the original mount location"
        );
        assert_ne!(
            anchor, moved_node,
            "cleanup must not detach the node Dioxus replaces with its reopening placeholder"
        );
        let placeholder = closed
            .edits
            .iter()
            .find_map(|mutation| match mutation {
                Mutation::CreatePlaceholder { id } => Some(*id),
                _ => None,
            })
            .expect("closing must create a reopening placeholder");

        visible.set(true);
        vdom.mark_dirty(ScopeId::APP);
        mounted = vdom.render_immediate_to_vec();
        assert!(
            mounted.edits.iter().any(|mutation| matches!(
                mutation, Mutation::ReplaceWith { id, .. } if *id == placeholder
            )),
            "reopening must reuse the placeholder without remounting the parent"
        );
    }
}

#[test]
fn closing_and_reopening_portal_keeps_renderer_replacement_attached() {
    fn menu() -> Element {
        let visible = use_context::<Rc<Cell<bool>>>();
        rsx! {
            if visible.get() {
                Portal { button { "Add mapping" } }
            } else {
                div { style: "display: none;", "closed menu" }
            }
        }
    }

    let visible = Rc::new(Cell::new(true));
    let document = Rc::new(RecordingDocument::default());
    let mut vdom = VirtualDom::new(menu);
    vdom.provide_root_context(Rc::clone(&visible));
    vdom.provide_root_context(document as Rc<dyn document::Document>);
    let mut mounted = vdom.rebuild_to_vec();

    for _ in 0..3 {
        vdom.render_immediate(&mut dioxus::core::NoOpMutations);
        let moved_node = mounted
            .edits
            .iter()
            .find_map(|edit| match edit {
                Mutation::SetAttribute {
                    name: "id",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } if value.starts_with("if-portal-") => Some(*id),
                _ => None,
            })
            .expect("mount must assign an id to the moved portal node");

        visible.set(false);
        vdom.mark_dirty(ScopeId::APP);
        let mut closed = Mutations::default();
        vdom.render_immediate(&mut closed);
        assert!(
            closed
                .edits
                .iter()
                .any(|edit| matches!(edit, Mutation::ReplaceWith { .. }))
        );
        assert!(
            !closed.edits.iter().any(|edit| matches!(edit,
                Mutation::ReplaceWith { id, .. } if *id == moved_node
            )),
            "closing must replace an attached anchor, not the portal node removed by cleanup: {:?}",
            closed.edits,
        );
        assert!(render(&vdom).contains("closed menu"));

        visible.set(true);
        vdom.mark_dirty(ScopeId::APP);
        mounted = Mutations::default();
        vdom.render_immediate(&mut mounted);
        assert!(render(&vdom).contains("Add mapping"));
    }
}
