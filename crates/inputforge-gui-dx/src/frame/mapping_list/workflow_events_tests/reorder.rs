use super::{
    document,
    fixture::{self, address},
};
use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use std::rc::Rc;
#[test]
fn handles_emit_pointer_gestures_with_group_and_row_boundaries() {
    let (dom, _, _) = fixture::mount();
    let html = dioxus_ssr::render(&dom);
    assert_eq!(html.matches("onpointerdown=\"").count(), 3);
    assert_eq!(html.matches("data-sortable-owner=\"").count(), 3);
    assert_eq!(html.matches("data-sortable-row=\"").count(), 3);
    assert_eq!(html.matches("data-sortable-group=\"").count(), 1);
}
#[test]
fn pointer_cleanup_targets_only_its_mount_and_source_handles() {
    let document = Rc::new(document::Document::default());
    let mut previous_owner = None;
    for cycle in 1..=2 {
        let (dom, _, _) = fixture::mount_with_document(Rc::<document::Document>::clone(&document));
        let html = scraper::Html::parse_fragment(&dioxus_ssr::render(&dom));
        let handles = html
            .select(&scraper::Selector::parse(".if-sortable-handle").unwrap())
            .collect::<Vec<_>>();
        let owner = handles[0].value().attr("data-sortable-owner").unwrap();
        assert_ne!(previous_owner.as_deref(), Some(owner));
        assert!(
            handles
                .iter()
                .all(|h| h.value().attr("data-sortable-owner") == Some(owner))
        );
        drop(dom);
        let cleanups = document.pointer_cleanups();
        assert_eq!(cleanups.len(), cycle * 4);
        assert_eq!(document.pointer_installs(), cycle);
        let current = &cleanups[(cycle - 1) * 4..];
        let owner = serde_json::json!(owner).to_string();
        assert!(
            current
                .iter()
                .any(|s| s.contains(&format!("?.owner === {owner}")))
        );
        for handle in handles {
            let key =
                serde_json::json!(handle.value().attr("data-sortable-handle").unwrap()).to_string();
            assert!(
                current
                    .iter()
                    .any(|s| s.contains(&format!("?.handle === {key}")))
            );
        }
        previous_owner = Some(serde_json::from_str::<String>(&owner).unwrap());
    }
}
#[test]
fn drag_hover_hint_clears_on_cancel_without_delivery() {
    let (mut dom, _, rx) = fixture::mount();
    let handles = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-handle");
    fixture::drag(&dom, "dragstart", handles[1]);
    dom.render_immediate_to_vec();
    let gaps = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-gap");
    fixture::drag(&dom, "dragover", gaps[3]);
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("if-sortable-gap--target"));
    fixture::drag(&dom, "dragend", handles[1]);
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(!html.contains("if-sortable-gap--target"));
    assert!(!html.contains("if-sortable--dragging"));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn removing_the_drag_source_clears_shared_state_while_the_list_stays_mounted() {
    let (mut dom, f, rx) = fixture::mount();
    let handles = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-handle");
    fixture::drag(&dom, "dragstart", handles[1]);
    dom.render_immediate_to_vec();
    let gaps = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-gap");
    fixture::drag(&dom, "dragover", gaps[3]);
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("if-sortable-gap--target"));
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&address(1), "Default");
    f.config
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
    for _ in 0..3 {
        dom.render_immediate_to_vec();
    }
    let html = dioxus_ssr::render(&dom);
    assert!(!html.contains("if-sortable-gap--target"));
    assert!(!html.contains("if-sortable--dragging"));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn keyboard_listener_delivers_selected_reorder_through_app_bindings() {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let document = Rc::new(document::Document::default());
        let (mut dom, f, rx) = fixture::mount_with_document(Rc::<document::Document>::clone(&document));
        let mut view = f.view.borrow().unwrap(); view.selected_mapping.set(Some(("Default".into(), address(1))));
        for _ in 0..4 { tokio::task::yield_now().await; dom.render_immediate_to_vec(); }
        document.keyboard("ArrowUp", 1,&format!("{:?}:Default",f.state.read().active_profile.as_ref().map(inputforge_core::profile::Profile::id)));
        for _ in 0..4 { tokio::task::yield_now().await; dom.render_immediate_to_vec(); }
        assert!(matches!(rx.try_recv().unwrap(), EngineCommand::ReorderMapping { input, target_index_in_group: 0, .. } if input == address(1)));
        assert!(dioxus_ssr::render(&dom).contains("Mapping moved to position 1 of 3 in buttons."));
    });
}
#[test]
fn drag_uses_captured_source_and_rejects_order_drift_before_drop() {
    for drift in [false, true] {
        let (mut dom, f, rx) = fixture::mount();
        let handles = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-handle");
        assert_eq!(handles.len(), 3);
        fixture::drag(&dom, "dragstart", handles[1]);
        dom.render_immediate_to_vec();
        if drift {
            f.state
                .write()
                .active_profile
                .as_mut()
                .unwrap()
                .reorder_mapping_in_group(&address(1), "Default", 0);
            f.config
                .borrow()
                .unwrap()
                .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
            for _ in 0..3 {
                dom.render_immediate_to_vec();
            }
        }
        let gaps = fixture::elements(&dom, dom.base_scope().root_node(), "if-sortable-gap");
        assert_eq!(gaps.len(), 4);
        fixture::drag(&dom, "dragover", gaps[3]);
        dom.render_immediate_to_vec();
        if drift {
            assert!(!dioxus_ssr::render(&dom).contains("if-sortable-gap--target"));
        } else {
            assert!(dioxus_ssr::render(&dom).contains("if-sortable-gap--target"));
        }
        fixture::drag(&dom, "drop", gaps[3]);
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("if-sortable-gap--target"));
        if drift {
            assert_eq!(
                rx.try_recv().unwrap_err(),
                std::sync::mpsc::TryRecvError::Empty
            );
        } else {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::ReorderMapping { input, target_index_in_group: 2, .. } if input == address(1))
            );
        }
    }
}
