//! Real rail handlers use app validation, delivery and navigation.
mod document;
mod fixture;
mod reorder;
use dioxus::prelude::*;
use fixture::{address, click, menu, mount};
use inputforge_core::engine::EngineCommand;
use std::{rc::Rc, sync::mpsc::TryRecvError};
#[test]
fn duplicate_to_mode_delivers_actions_or_opens_current_collision() {
    for collision in [false, true] {
        let (mut dom, f, rx) = mount();
        if collision {
            f.state
                .write()
                .active_profile
                .as_mut()
                .unwrap()
                .set_mapping(&address(0), "Other", None, vec![]);
        }
        menu(&mut dom, "First");
        click(&dom, "Other");
        dom.render_immediate_to_vec();
        if collision {
            assert_eq!(*f.view.borrow().unwrap().editing_mode.peek(), "Other");
            assert_eq!(
                *f.view.borrow().unwrap().selected_mapping.peek(),
                Some(("Other".into(), address(0)))
            );
            assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
        } else {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { input, mode, name, actions } if input == address(0) && mode == "Other" && name.as_deref() == Some("First") && actions == vec![inputforge_core::action::Action::Invert])
            );
            assert!(matches!(
                rx.try_recv().unwrap(),
                EngineCommand::MappingEditBarrier { .. }
            ));
        }
    }
}
#[test]
fn failed_duplicate_keeps_menu_and_exposes_feedback() {
    let (mut dom, _, rx) = mount();
    menu(&mut dom, "First");
    drop(rx);
    click(&dom, "Other");
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("if-row-menu"));
    assert!(html.contains(r#"role="alert""#));
}
#[test]
fn menu_reorder_announces_only_successful_delivery() {
    for failed in [false, true] {
        let (mut dom, _, rx) = mount();
        menu(&mut dom, "Second");
        let rx = if failed {
            drop(rx);
            None
        } else {
            Some(rx)
        };
        click(&dom, "Move up");
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        if let Some(rx) = rx {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::ReorderMapping { input, target_index_in_group: 0, .. } if input == address(1))
            );
            assert!(html.contains("Mapping moved to position 1 of 3 in buttons."));
        } else {
            assert!(!html.contains("Mapping moved to position"));
            assert!(html.contains(r#"role="alert""#));
        }
    }
}
#[test]
fn duplicate_source_deletion_closes_pad_and_releases_capture() {
    let (mut dom, f, rx) = mount();
    menu(&mut dom, "First");
    click(&dom, "Duplicate");
    for _ in 0..4 {
        dom.render_immediate_to_vec();
    }
    assert!(dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&address(0), "Default");
    f.config
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
    for _ in 0..4 {
        dom.render_immediate_to_vec();
    }
    assert!(!dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
    dom.in_scope(ScopeId::APP, || {
        assert!(
            !*consume_context::<crate::patterns::live_capture::LiveCapture>()
                .active
                .peek()
        );
    });
    assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
}
#[test]
fn rail_listeners_install_once_and_remove_synchronously_on_unmount() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let document = Rc::new(document::Document::default());
            for cycle in 1..=2 {
                let (mut dom, _, _rx) =
                    fixture::mount_with_document(Rc::<document::Document>::clone(&document));
                for _ in 0..4 {
                    tokio::task::yield_now().await;
                    dom.render_immediate_to_vec();
                }
                assert_eq!(document.installs(), cycle * 2);
                drop(dom);
                assert_eq!(document.cleanups(), cycle * 2);
                assert_eq!(document.shutdowns(), cycle * 2);
            }
        });
}
#[test]
fn delete_confirmation_dispatches_only_after_confirmation_and_retains_failed_dialog() {
    for failed in [false, true] {
        let (mut dom, f, rx) = mount();
        menu(&mut dom, "First");
        click(&dom, "Delete");
        for _ in 0..3 {
            dom.render_immediate_to_vec();
        }
        assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
        let rx = if failed {
            drop(rx);
            None
        } else {
            Some(rx)
        };
        click(&dom, "Delete");
        for _ in 0..3 {
            dom.render_immediate_to_vec();
        }
        if let Some(rx) = rx {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::RemoveMapping { input, mode } if input == address(0) && mode == "Default")
            );
            assert!(matches!(
                rx.try_recv().unwrap(),
                EngineCommand::MappingEditBarrier { .. }
            ));
        } else {
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("Delete mapping"));
            assert!(html.contains(r#"role="alert""#));
        }
        assert!(
            f.state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .find_mapping(&address(0), "Default")
                .is_some()
        );
    }
}

#[test]
fn duplicate_arm_cannot_migrate_to_another_mode_before_its_effect_runs() {
    let (mut dom, f, rx) = mount();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&address(0), "Other", Some("First".into()), vec![]);
    menu(&mut dom, "First");
    click(&dom, "Duplicate");
    let mut view = f.view.borrow().unwrap();
    view.editing_mode.set("Other".into());
    for _ in 0..4 {
        dom.render_immediate_to_vec();
    }
    assert!(!dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
    assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
}
#[test]
fn stale_menu_move_observes_a_new_device_filter_before_rerender() {
    let (mut dom, f, rx) = mount();
    menu(&mut dom, "Second");
    let label = f
        .config
        .borrow()
        .unwrap()
        .peek()
        .device_display_name(&inputforge_core::types::DeviceId("rail".into()));
    click(&dom, &label);
    click(&dom, "Move up");
    assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[test]
fn an_open_menu_never_retargets_a_replaced_mapping_after_a_new_frame() {
    let (mut dom, f, rx) = mount();
    menu(&mut dom, "First");
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(
            &address(0),
            "Default",
            Some("First".into()),
            vec![inputforge_core::action::Action::MapToMouse {
                target: inputforge_core::action::MouseTarget::LeftButton,
                behavior: inputforge_core::action::OutputBehavior::Hold,
            }],
        );
    let next = inputforge_app::bridge::prepare(&f.state.read(), None).config;
    dom.in_runtime(|| f.config.borrow().unwrap().set(next));
    dom.render_immediate_to_vec();
    click(&dom, "Other");
    dom.render_immediate_to_vec();
    assert!(
        rx.try_recv().is_err(),
        "old menu must not target replacement mapping"
    );
}
