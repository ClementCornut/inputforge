#[path = "navigation/document.rs"]
mod document;
#[path = "navigation/fixture.rs"]
mod fixture;
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, Condition, OutputBehavior},
    engine::EngineCommand,
    profile::Profile,
    types::{DeviceId, InputAddress, InputId, KeyCombo, PhysicalKey},
};
use inputforge_gui_dx::{
    PanelSlot,
    mapping_edits::{StageId, StageIdSegment},
    navigation::{
        FilterRequest, Intent, RailPublication, StageIntent, StageMenu, StageRequest, Surface,
    },
    snapshots::{ConfigSnapshot, GlyphFlags, MappingSummary},
};
use std::{cell::RefCell, rc::Rc, sync::mpsc};
fn key() -> (String, InputAddress) {
    ("Default".into(), InputAddress::Unbound)
}
fn path() -> StageId {
    StageId(vec![StageIdSegment::Index(0)])
}
#[test]
fn same_name_profile_switch_resets_selection_and_mode_but_rename_does_not() {
    let mut f = fixture::mount();
    f.nav.editing_mode.set("Other".into());
    f.nav
        .selected_mapping
        .set(Some(("Other".into(), InputAddress::Unbound)));
    f.state.write().active_profile = Some(fixture::profile());
    f.bindings.navigation.reconcile.call(());
    assert_eq!(*f.nav.editing_mode.peek(), "Default");
    assert!(f.nav.selected_mapping.peek().is_none());
    f.nav.selected_mapping.set(Some(key()));
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_name("Renamed".into());
    f.bindings.navigation.reconcile.call(());
    assert!(f.nav.selected_mapping.peek().is_some());
}
#[test]
fn app_transitions_toggle_live_panel_and_close_for_batch() {
    let f = fixture::mount();
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Settings));
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::Settings);
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Settings));
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::None);
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Devices));
    f.bindings
        .navigation
        .dispatch
        .call(Intent::Surface(Surface::BulkMap));
    assert_eq!(*f.nav.main_surface.peek(), Surface::BulkMap);
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::None);
}
#[test]
fn reconciliation_removes_missing_selection_and_falls_back_missing_mode() {
    let mut f = fixture::mount();
    f.nav.selected_mapping.set(Some((
        "Default".into(),
        InputAddress::Bound {
            device: DeviceId("gone".into()),
            input: InputId::Button { index: 1 },
        },
    )));
    f.bindings.navigation.reconcile.call(());
    assert!(f.nav.selected_mapping.peek().is_none());
    f.nav.editing_mode.set("Gone".into());
    f.bindings.navigation.reconcile.call(());
    assert_eq!(*f.nav.editing_mode.peek(), "Default");
}
#[test]
fn disposed_bindings_do_not_navigate_and_empty_devices_gate_is_preserved() {
    let f = fixture::mount();
    f.state.write().active_profile = None;
    f.bindings.navigation.reconcile.call(());
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Devices));
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::None);
    f.bindings
        .navigation
        .dispatch
        .call(Intent::Surface(Surface::BulkMap));
    assert_eq!(*f.nav.main_surface.peek(), Surface::Mappings);
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Profiles));
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::Profiles);
    f.bindings.dispose.call(());
    f.bindings
        .navigation
        .dispatch
        .call(Intent::TogglePanel(PanelSlot::Settings));
    assert_eq!(*f.nav.panel_slot.peek(), PanelSlot::Profiles);
}
fn summary(name: &str, input: InputAddress) -> MappingSummary {
    MappingSummary {
        input,
        mode: "Default".into(),
        name: Some(name.into()),
        glyphs: GlyphFlags::default(),
        referenced_devices: vec![],
        first_vjoy_output: None,
    }
}
#[test]
fn filter_projection_matches_names_without_case_sensitivity() {
    let f = fixture::mount();
    let config = ConfigSnapshot {
        mappings: vec![summary("Boost", InputAddress::Unbound)],
        ..ConfigSnapshot::default()
    };
    let view = f.bindings.navigation.filter.call(FilterRequest {
        config,
        mode: "Default".into(),
        query: "BOO".into(),
        device: None,
    });
    assert_eq!(view.total, 1);
    assert_eq!(view.rows.len(), 1);
}
#[test]
fn selection_changes_clear_expansion_menu_and_diagnostics() {
    let mut f = fixture::mount();
    f.nav.selected_mapping.set(Some(key()));
    f.bindings.navigation.reconcile.call(());
    f.editor.expanded.write().insert(path());
    f.editor.hints.write().insert(path(), "old".into());
    f.editor.menu.set(Some(StageMenu {
        stage: path(),
        x: 0.,
        y: 0.,
    }));
    f.nav.selected_mapping.set(None);
    f.bindings.navigation.reconcile.call(());
    assert!(f.editor.expanded.peek().is_empty());
    assert!(f.editor.hints.peek().is_empty());
    assert!(f.editor.menu.peek().is_none());
}
#[test]
fn old_profile_row_event_cannot_select_same_key_on_replacement() {
    let f = fixture::mount();
    let old = f.state.read().active_profile.as_ref().unwrap().id().clone();
    f.state.write().active_profile = Some(fixture::profile());
    f.bindings.navigation.dispatch.call(Intent::Select {
        profile: Some(old),
        key: key(),
    });
    assert!(f.nav.selected_mapping.peek().is_none());
}
#[test]
fn retained_stage_toggle_is_rejected_after_same_name_profile_replacement() {
    let mut f = fixture::mount();
    f.nav.selected_mapping.set(Some(key()));
    f.bindings.navigation.reconcile.call(());
    let request = StageRequest {
        actions: vec![Action::Invert],
        profile: f
            .state
            .read()
            .active_profile
            .as_ref()
            .map(Profile::id)
            .cloned(),
        key: key(),
        path: path(),
        intent: StageIntent::Toggle,
    };
    f.state.write().active_profile = Some(fixture::profile());
    f.bindings.navigation.stage.call(request);
    assert!(f.editor.expanded.peek().is_empty());
}
#[test]
fn pending_added_mode_survives_reconciliation_before_engine_consumes_the_queue() {
    let f = fixture::mount();
    let scope = inputforge_gui_dx::mode_management::Scope {
        profile: f
            .state
            .read()
            .active_profile
            .as_ref()
            .map(Profile::id)
            .cloned(),
        editing: "Default".into(),
        revision: f.state.read().mode_revision,
    };
    f.bindings
        .modes
        .dispatch
        .call(inputforge_gui_dx::mode_management::Request {
            scope,
            operation: inputforge_gui_dx::mode_management::Operation::Add {
                raw: "New".into(),
                blur: false,
            },
        })
        .unwrap();
    assert!(matches!(
        f.rx.try_recv().unwrap(),
        EngineCommand::AddMode { .. }
    ));
    for _ in 0..3 {
        f.bindings.navigation.reconcile.call(());
        assert_eq!(*f.nav.editing_mode.peek(), "New");
    }
}
#[test]
fn keyboard_rows_follow_the_visible_axis_button_hat_group_order() {
    let f = fixture::mount();
    let mut config = ConfigSnapshot::default();
    for (name, input) in [
        ("Hat", InputId::Hat { index: 0 }),
        ("First button", InputId::Button { index: 1 }),
        ("Axis", InputId::Axis { index: 0 }),
        ("Second button", InputId::Button { index: 0 }),
    ] {
        config.mappings.push(summary(
            name,
            InputAddress::Bound {
                device: DeviceId("pad".into()),
                input,
            },
        ));
    }
    let view = f.bindings.navigation.filter.call(FilterRequest {
        config,
        mode: "Default".into(),
        query: String::new(),
        device: None,
    });
    assert_eq!(
        view.rows
            .iter()
            .map(|r| r.name.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["Axis", "First button", "Second button", "Hat"]
    );
}
#[test]
fn nested_shortcut_uses_guarded_edit_and_old_listener_cannot_navigate_after_close() {
    fixture::runtime().block_on(async {
  let mut f=fixture::mount();let branch=Action::Conditional{condition:Condition::ButtonPressed{input:InputAddress::Unbound},if_true:vec![Action::Invert,Action::MapToKeyboard{key:KeyCombo{key:PhysicalKey::F11,modifiers:vec![]},behavior:OutputBehavior::Hold}],if_false:vec![]};
  f.state.write().active_profile.as_mut().unwrap().set_mapping(&InputAddress::Unbound,"Default",None,vec![branch]);f.nav.selected_mapping.set(Some(key()));f.bindings.navigation.reconcile.call(());
  let document=Rc::new(document::Document::default());f.dom.provide_root_context(Rc::clone(&document) as Rc<dyn dioxus::prelude::document::Document>);
  let listener=f.dom.in_scope(ScopeId::ROOT,||f.bindings.navigation.editor.call(f.editor));
  let rendered=format!("{:?}",f.state.read().active_profile.as_ref().unwrap().find_mapping(&InputAddress::Unbound,"Default").unwrap().actions);
  let context=format!("{:?}:{:?}",f.state.read().active_profile.as_ref().map(Profile::id),f.nav.selected_mapping.peek().as_ref());
  document.event(0,serde_json::json!(["ArrowUp",0,0,1,"Stage","0.T.1",4,9,context,rendered]));fixture::drive(&mut f.dom).await;
  assert!(matches!(f.rx.try_recv().unwrap(),EngineCommand::SetMapping{actions,..} if matches!(&actions[0],Action::Conditional{if_true,..} if matches!(if_true[0],Action::MapToKeyboard{..}))));
  assert!(matches!(f.rx.try_recv().unwrap(),EngineCommand::MappingEditBarrier{..}));listener.close.call(());assert_eq!(document.cleanups(),1);assert_eq!(document.shutdowns(),1);
  document.event(0,serde_json::json!(["z",1,0,0,"Editor","",0,0,context,rendered]));fixture::drive(&mut f.dom).await;assert_eq!(f.rx.try_recv().unwrap_err(),mpsc::TryRecvError::Empty);
 });
}
#[test]
fn obsolete_or_closed_rail_events_cannot_request_focus() {
    fixture::runtime().block_on(async {
        let mut f = fixture::mount();
        f.nav.selected_mapping.set(Some(key()));
        f.bindings.navigation.reconcile.call(());
        let document = Rc::new(document::Document::default());
        f.dom.provide_root_context(
            Rc::clone(&document) as Rc<dyn dioxus::prelude::document::Document>
        );
        let requests = Rc::new(RefCell::new(Vec::new()));
        let focus = Rc::clone(&requests);
        let frame = inputforge_app::bridge::prepare(&f.state.read(), None);
        let publication = f.dom.in_scope(ScopeId::ROOT, || {
            let config = Signal::new(frame.config);
            let rows = Memo::new(move || {
                config
                    .read()
                    .mappings
                    .iter()
                    .map(|m| (m.mode.clone(), m.input.clone()))
                    .collect()
            });
            RailPublication {
                query: Signal::new(String::new()),
                focused: Signal::new(false),
                device: Signal::new(None),
                rows,
                config,
                meta: Signal::new(frame.meta),
                capture: Signal::new(false),
                feedback: Signal::new(None),
                announcement: Signal::new(String::new()),
                focus: Callback::new(move |selector| focus.borrow_mut().push(selector)),
            }
        });
        let listener = f.dom.in_scope(ScopeId::ROOT, || {
            f.bindings.navigation.rail.call(publication)
        });
        let context = format!(
            "{:?}:Default",
            f.state.read().active_profile.as_ref().map(Profile::id)
        );
        document.event(0, serde_json::json!(["Enter", 0, 0, 0, context]));
        fixture::drive(&mut f.dom).await;
        assert_eq!(*requests.borrow(), vec!["[data-editor-focus]"]);
        f.state.write().active_profile = Some(fixture::profile());
        document.event(0, serde_json::json!(["Enter", 0, 0, 0, context]));
        fixture::drive(&mut f.dom).await;
        assert_eq!(requests.borrow().len(), 1);
        listener.close.call(());
        document.event(0, serde_json::json!(["f", 0, 1, 0, context]));
        fixture::drive(&mut f.dom).await;
        assert_eq!(requests.borrow().len(), 1);
    });
}
#[path = "navigation/stale.rs"]
mod stale;
