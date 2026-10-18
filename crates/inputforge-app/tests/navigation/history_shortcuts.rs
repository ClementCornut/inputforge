//! Browser transport payloads restore selected mapping history through the app listener.
use super::*;
use inputforge_gui_dx::mapping_edits::{Edit, Request};

#[test]
fn history_shortcut_payloads_reach_undo_and_both_redo_bindings() {
    fixture::runtime().block_on(async {
        let mut f = fixture::mount();
        f.nav.selected_mapping.set(Some(key()));
        f.bindings.navigation.reconcile.call(());
        let before = f.state.read().active_profile.as_ref().unwrap()
            .find_mapping(&key().1, &key().0).unwrap().clone();
        f.bindings.edit.call(Request {
            profile: f.state.read().active_profile.as_ref().map(Profile::id).cloned(),
            before,
            edit: Edit::Rename("Changed".into()),
            label: "Rename".into(), group: None, editor: true,
        }).unwrap();
        assert!(matches!(f.rx.try_recv().unwrap(), EngineCommand::SetMapping {name: Some(name), ..} if name == "Changed"));
        assert!(matches!(f.rx.try_recv().unwrap(), EngineCommand::MappingEditBarrier {..}));
        let document = Rc::new(document::Document::default());
        f.dom.provide_root_context(Rc::clone(&document) as Rc<dyn dioxus::prelude::document::Document>);
        let listener = f.dom.in_scope(ScopeId::ROOT, || f.bindings.navigation.editor.call(f.editor));
        let context = format!("{:?}:{:?}", f.state.read().active_profile.as_ref().map(Profile::id), f.nav.selected_mapping.peek().as_ref());
        for (key, shift, expected) in [("z", 0, None), ("Z", 1, Some("Changed")), ("z", 0, None), ("y", 0, Some("Changed"))] {
            document.event(0, serde_json::json!([key, 1, shift, 0, "Editor", "", 0, 0, context, "[Invert]"]));
            fixture::drive(&mut f.dom).await;
            assert!(matches!(f.rx.try_recv().unwrap(), EngineCommand::SetMapping {name, actions, ..} if name.as_deref() == expected && actions == vec![Action::Invert]));
            assert!(matches!(f.rx.try_recv().unwrap(), EngineCommand::MappingEditBarrier {..}));
        }
        for (key, shift) in [("z", 0), ("Z", 1), ("y", 0)] {
            document.event(0, serde_json::json!([key, 1, shift, 0, "Input", "", 0, 0, context, "[Invert]"]));
            fixture::drive(&mut f.dom).await;
            assert_eq!(f.rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        }
        listener.close.call(());
    });
}
