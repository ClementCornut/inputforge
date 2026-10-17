//! Actual rendered instrument handlers use the app's captured targets and drafts.
mod document;
mod dom;
mod fixture;
use dioxus::{html::SerializedHtmlEventConverter, prelude::*};
use fixture::{fixture, settle};
use inputforge_core::{
    action::Action,
    engine::EngineCommand,
    processing::{curves::ResponseCurve, deadzone::DeadzoneConfig},
};
fn curve() -> Action {
    Action::ResponseCurve {
        curve: ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 0.5)], false).unwrap(),
    }
}
fn runtime(f: impl Future<Output = ()>) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f);
}
#[test]
fn rendered_curve_keys_toolbar_and_drag_use_app_history_and_named_mapping() {
    runtime(async {
        let (mut dom, f, _doc, rx) = fixture(curve());
        dom::key(&dom, "response curve", Key::End, Code::End);
        dom::key(&dom, "response curve", Key::ArrowUp, Code::ArrowUp);
        dom.render_immediate_to_vec();
        let EngineCommand::SetMapping { name, actions, .. } = rx.try_recv().unwrap() else {
            panic!("expected mapping");
        };
        assert_eq!(name.as_deref(), Some("Keep"));
        assert!(
            matches!(&actions[0],Action::ResponseCurve{curve:ResponseCurve::PiecewiseLinear{points,..}} if (points[1].1-0.51).abs()<1e-12)
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::MappingEditBarrier { .. }
        ));
        dom::click(&dom, "Spline");
        dom.render_immediate_to_vec();
        assert!(
            matches!(rx.try_recv().unwrap(),EngineCommand::SetMapping{actions,..} if matches!(&actions[0],Action::ResponseCurve{curve:ResponseCurve::CubicSpline{..}}))
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::MappingEditBarrier { .. }
        ));
        assert_eq!(
            f.editor.borrow().unwrap().undo_log.peek().stacks[&fixture::key()]
                .undo
                .len(),
            2
        );
    });
}
#[test]
fn rendered_deadzone_numeric_handler_keeps_failed_draft_and_alert() {
    runtime(async {
        let (mut dom, f, _doc, rx) = fixture(Action::Deadzone {
            config: DeadzoneConfig::default(),
        });
        settle(&mut dom).await;
        drop(rx);
        dom::send(
            &dom,
            "dz-low",
            "input",
            PlatformEventData::new(Box::new(SerializedFormData::new("-0.8".into(), vec![]))),
        );
        dom.render_immediate_to_vec();
        dom::key(&dom, "dz-low", Key::Enter, Code::Enter);
        dom::send(
            &dom,
            "dz-low",
            "blur",
            PlatformEventData::new(Box::new(SerializedFocusData {})),
        );
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(html.contains("value=\"-0.8\""), "{html}");
        assert!(f.editor.borrow().unwrap().error.peek().is_some());
        assert!(f.editor.borrow().unwrap().undo_log.peek().stacks.is_empty());
    });
}
#[test]
fn rendered_pointer_owner_closes_synchronously_and_remount_cannot_commit_old_release() {
    runtime(async {
        let (mut dom, f, doc, rx) = fixture(curve());
        dom::send(
            &dom,
            "response curve",
            "mounted",
            PlatformEventData::new(Box::new(MountedData::new(()))),
        );
        settle(&mut dom).await;
        assert_eq!(doc.installs(), 1);
        doc.pointer(0, "down", 195.0, 52.0);
        doc.pointer(0, "move", 195.0, 80.0);
        settle(&mut dom).await;
        assert!(dioxus_ssr::render(&dom).contains("data-dragging=\"true\""));
        f.visible.borrow().unwrap().set(false);
        dom.render_immediate_to_vec();
        assert_eq!(doc.cleanups(), 1);
        assert_eq!(doc.shutdowns(), 1);
        doc.pointer(0, "up", 195.0, 80.0);
        f.visible.borrow().unwrap().set(true);
        dom.render_immediate_to_vec();
        settle(&mut dom).await;
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
        assert!(!dioxus_ssr::render(&dom).contains("data-dragging=\"true\""));
    });
}
