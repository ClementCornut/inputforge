use super::*;
async fn repeated_stage_position(applied: bool) {
    let mut f = fixture::mount();
    let original = vec![
        Action::Invert,
        Action::MapToKeyboard {
            key: KeyCombo {
                key: PhysicalKey::F11,
                modifiers: vec![],
            },
            behavior: OutputBehavior::Hold,
        },
    ];
    let rendered = format!("{original:?}");
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&InputAddress::Unbound, "Default", None, original);
    f.nav.selected_mapping.set(Some(key()));
    f.bindings.navigation.reconcile.call(());
    let document = Rc::new(document::Document::default());
    f.dom
        .provide_root_context(Rc::clone(&document) as Rc<dyn dioxus::prelude::document::Document>);
    let listener = f.dom.in_scope(ScopeId::ROOT, || {
        f.bindings.navigation.editor.call(f.editor)
    });
    let context = format!(
        "{:?}:{:?}",
        f.state.read().active_profile.as_ref().map(Profile::id),
        f.nav.selected_mapping.peek().as_ref()
    );
    let payload = serde_json::json!(["ArrowUp", 0, 0, 1, "Stage", "1", 0, 0, context, rendered]);
    document.event(0, payload.clone());
    fixture::drive(&mut f.dom).await;
    let EngineCommand::SetMapping { actions, .. } = f.rx.try_recv().unwrap() else {
        panic!("first stage move expected")
    };
    let EngineCommand::MappingEditBarrier { owner, serial } = f.rx.try_recv().unwrap() else {
        panic!("queue barrier expected")
    };
    if applied {
        let mut state = f.state.write();
        state.active_profile.as_mut().unwrap().set_mapping(
            &InputAddress::Unbound,
            "Default",
            None,
            actions,
        );
        state.mapping_edit_barrier = Some((owner, serial));
    }
    document.event(0, payload);
    fixture::drive(&mut f.dom).await;
    assert_eq!(f.rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    document.event(
        0,
        serde_json::json!(["z", 1, 0, 0, "Editor", "", 0, 0, context, rendered]),
    );
    fixture::drive(&mut f.dom).await;
    assert!(
        matches!(f.rx.try_recv().unwrap(),EngineCommand::SetMapping{actions,..} if matches!(actions[0],Action::Invert)&&matches!(actions[1],Action::MapToKeyboard{..}))
    );
    listener.close.call(());
}
#[test]
fn queued_stage_move_rejects_the_old_position_before_dom_publication() {
    fixture::runtime().block_on(repeated_stage_position(false));
}
#[test]
fn executed_stage_move_rejects_the_old_position_before_dom_publication() {
    fixture::runtime().block_on(repeated_stage_position(true));
}
#[test]
fn pending_mode_rename_rejects_retained_old_mode_row_after_reconciliation() {
    let mut f = fixture::mount();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&InputAddress::Unbound, "Other", None, vec![Action::Invert]);
    f.nav.editing_mode.set("Other".into());
    f.bindings.navigation.reconcile.call(());
    let profile = f
        .state
        .read()
        .active_profile
        .as_ref()
        .map(Profile::id)
        .cloned();
    let scope = inputforge_gui_dx::mode_management::Scope {
        profile: profile.clone(),
        editing: "Other".into(),
        revision: f.state.read().mode_revision,
    };
    f.bindings
        .modes
        .dispatch
        .call(inputforge_gui_dx::mode_management::Request {
            scope,
            operation: inputforge_gui_dx::mode_management::Operation::Rename {
                from: "Other".into(),
                raw: "New".into(),
                blur: false,
            },
        })
        .unwrap();
    f.bindings.navigation.reconcile.call(());
    assert_eq!(*f.nav.editing_mode.peek(), "New");
    f.bindings.navigation.dispatch.call(Intent::Select {
        profile,
        key: ("Other".into(), InputAddress::Unbound),
    });
    assert!(f.nav.selected_mapping.peek().is_none());
    f.bindings.navigation.reconcile.call(());
    assert_eq!(*f.nav.editing_mode.peek(), "New");
    assert!(f.nav.selected_mapping.peek().is_none());
}
#[test]
fn retained_stage_header_cannot_expand_another_stage_after_queued_reorder() {
    let mut f = fixture::mount();
    let actions = vec![
        Action::Invert,
        Action::MapToKeyboard {
            key: KeyCombo {
                key: PhysicalKey::F11,
                modifiers: vec![],
            },
            behavior: OutputBehavior::Hold,
        },
    ];
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&InputAddress::Unbound, "Default", None, actions.clone());
    f.nav.selected_mapping.set(Some(key()));
    f.bindings.navigation.reconcile.call(());
    let profile = f
        .state
        .read()
        .active_profile
        .as_ref()
        .map(Profile::id)
        .cloned();
    let captured = StageRequest {
        profile: profile.clone(),
        key: key(),
        path: StageId(vec![StageIdSegment::Index(1)]),
        actions: actions.clone(),
        intent: StageIntent::Toggle,
    };
    f.bindings
        .edit
        .call(inputforge_gui_dx::mapping_edits::Request {
            profile,
            before: inputforge_core::action::Mapping {
                input: InputAddress::Unbound,
                mode: "Default".into(),
                name: None,
                actions,
            },
            edit: inputforge_gui_dx::mapping_edits::Edit::Step {
                path: StageId(vec![StageIdSegment::Index(1)]),
                down: false,
            },
            label: "Move".into(),
            group: None,
            editor: true,
        })
        .unwrap();
    f.bindings.navigation.stage.call(captured);
    assert!(f.editor.expanded.peek().is_empty());
}
