//! Device inspector regressions through application bindings.
#[path = "devices/fixture.rs"]
mod fixture;
use dioxus::prelude::*;
use inputforge_gui_dx::controllers::inspector::Intent;
#[test]
fn selection_falls_back_to_connected_then_remembered_and_removed_row() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        assert_eq!(f.view.peek().selected.as_ref().unwrap().device_id.0, "live");
        f.send(Intent::Select(fixture::id("old")));
        assert_eq!(f.view.peek().draft, "Old alias");
        f.state.write().device_registry.remove(&fixture::id("old"));
        f.c.refresh();
        assert_eq!(f.view.peek().selected.as_ref().unwrap().device_id.0, "live");
        f.state.write().devices.clear();
        f.c.refresh();
        assert!(f.view.peek().selected.is_none());
    });
}
#[test]
fn alias_trim_noop_cancel_and_failed_send_retain_draft() {
    for connected in [true, false] {
        let (dom, f) = fixture::fixture(connected);
        dom.in_scope(ScopeId::ROOT, || {
            assert!(f.send(Intent::Select(fixture::id("old"))));
            assert!(f.send(Intent::Input(" Old alias ".into())));
            assert!(f.send(Intent::Save));
            f.rx.try_recv().unwrap_err();
            assert!(f.send(Intent::Input(" New alias ".into())));
            assert_eq!(f.send(Intent::Save), connected);
            assert_eq!(f.view.peek().draft, " New alias ");
            assert_eq!(f.view.peek().error.is_some(), !connected);
            if connected {
                let inputforge_core::engine::EngineCommand::Device(request) =
                    f.rx.try_recv().unwrap()
                else {
                    panic!("guarded alias request expected")
                };
                assert_eq!(
                    request.action,
                    inputforge_core::engine::DeviceAction::Alias(Some("New alias".into()))
                );
            }
            assert!(f.send(Intent::Cancel));
            assert_eq!(f.view.peek().draft, "Old alias");
        });
    }
}
#[test]
fn retained_callbacks_reject_changed_profile_device_and_unmount() {
    for change in 0..4 {
        let (dom, f) = fixture::fixture(true);
        dom.in_scope(ScopeId::ROOT, || {
            f.send(Intent::Select(fixture::id("old")));
            f.send(Intent::Input("retained".into()));
            let workflow = f.view.peek().workflow;
            match change {
                0 => f.state.write().active_profile = Some(fixture::profile()),
                1 => f.state.write().profile_path = Some("/elsewhere.toml".into()),
                2 => {
                    f.state
                        .write()
                        .device_registry
                        .get_mut(&fixture::id("old"))
                        .unwrap()
                        .info
                        .instance_path = Some("replacement".into());
                }
                _ => f.bindings.unmount.call(f.owner),
            }
            assert!(!f.bindings.dispatch.call((f.owner, workflow, Intent::Save)));
            assert_eq!(f.view.peek().draft, "retained");
            f.rx.try_recv().unwrap_err();
        });
    }
}
#[test]
fn removal_rechecks_connection_and_reference_changes_before_delivery() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        assert!(!f.send(Intent::Remove));
        f.rx.try_recv().unwrap_err();
        f.send(Intent::Select(fixture::id("old")));
        assert!(f.view.peek().removal_reason.is_none());
        f.state
            .write()
            .active_profile
            .as_mut()
            .unwrap()
            .set_mapping(
                &inputforge_core::types::InputAddress::Bound {
                    device: fixture::id("old"),
                    input: inputforge_core::types::InputId::Button { index: 0 },
                },
                "Default",
                None,
                vec![],
            );
        assert!(!f.send(Intent::Remove));
        f.rx.try_recv().unwrap_err();
        assert!(f.view.peek().error.is_some());
    });
}
#[test]
fn old_ledger_selection_rejects_replacement_until_fresh_reconciliation() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        let workflow = f.view.peek().workflow;
        f.state.write().active_profile = Some(fixture::profile());
        assert!(
            !f.bindings
                .dispatch
                .call((f.owner, workflow, Intent::Select(fixture::id("old"))))
        );
        f.c.refresh();
        assert!(
            !f.bindings
                .dispatch
                .call((f.owner, workflow, Intent::Select(fixture::id("old"))))
        );
        assert!(f.send(Intent::Select(fixture::id("old"))));
    });
}
#[test]
fn empty_reconciliation_does_not_publish_new_workflows() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        f.state.write().devices.clear();
        f.state.write().device_registry.clear();
        f.c.refresh();
        let before = f.view.peek().clone();
        f.c.refresh();
        f.c.refresh();
        assert_eq!(*f.view.peek(), before);
    });
}
#[test]
fn keyboard_handling_leaves_ordinary_typing_native_and_handles_failed_enter() {
    let (dom, f) = fixture::fixture(false);
    dom.in_scope(ScopeId::ROOT, || {
        let workflow = f.view.peek().workflow;
        assert!(!f.bindings.key.call((f.owner, workflow, "a".into())));
        f.send(Intent::Input("draft".into()));
        assert!(f.bindings.key.call((f.owner, workflow, "Enter".into())));
        assert!(f.view.peek().error.is_some());
        assert_eq!(f.view.peek().draft, "draft");
        assert!(f.bindings.key.call((f.owner, workflow, "Escape".into())));
        assert_eq!(f.view.peek().draft, "");
    });
}
#[test]
fn remembered_fallback_and_empty_alias_use_current_native_target() {
    use inputforge_core::engine::{DeviceAction, EngineCommand};
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        f.state.write().devices.clear();
        f.c.refresh();
        assert_eq!(f.view.peek().selected.as_ref().unwrap().device_id.0, "old");
        assert!(f.send(Intent::Input("   ".into())));
        assert!(f.send(Intent::Save));
        let EngineCommand::Device(request) = f.rx.try_recv().unwrap() else {
            panic!("guarded device request expected")
        };
        assert_eq!(request.action, DeviceAction::Alias(None));
        assert_eq!(request.target.info.instance_path.as_deref(), Some("old"));
        f.state
            .write()
            .device_registry
            .get_mut(&fixture::id("old"))
            .unwrap()
            .info
            .instance_path = Some("replacement".into());
        f.c.refresh();
        assert!(!f.send(Intent::Save));
        assert_eq!(f.view.peek().draft, "   ");
        assert!(f.send(Intent::Select(fixture::id("old"))));
        assert_eq!(f.view.peek().draft, "Old alias");
    });
}
#[test]
fn nested_secondary_references_and_connection_changes_disable_removal() {
    use inputforge_core::{
        action::{Action, Condition},
        state::DeviceState,
        types::{InputAddress, InputId},
    };
    for connected in [false, true] {
        let (dom, f) = fixture::fixture(true);
        dom.in_scope(ScopeId::ROOT, || {
            f.send(Intent::Select(fixture::id("old")));
            if connected {
                let info = f.state.read().device_registry[&fixture::id("old")]
                    .info
                    .clone();
                f.state.write().devices.push(DeviceState {
                    info,
                    connected: true,
                    diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
                });
            } else {
                f.state
                    .write()
                    .active_profile
                    .as_mut()
                    .unwrap()
                    .set_mapping(
                        &InputAddress::Unbound,
                        "Default",
                        None,
                        vec![Action::Conditional {
                            condition: Condition::Not {
                                condition: Box::new(Condition::ButtonPressed {
                                    input: InputAddress::Bound {
                                        device: fixture::id("old"),
                                        input: InputId::Button { index: 0 },
                                    },
                                }),
                            },
                            if_true: vec![],
                            if_false: vec![],
                        }],
                    );
            }
            assert!(!f.send(Intent::Remove));
            f.rx.try_recv().unwrap_err();
            f.c.refresh();
            assert!(f.view.peek().removal_reason.is_some());
        });
    }
}
