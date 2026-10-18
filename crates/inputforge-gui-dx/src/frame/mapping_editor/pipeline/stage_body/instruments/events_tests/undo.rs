//! Restore publication reaches mounted instrument views before the engine frame arrives.
use super::*;
use std::sync::mpsc::Receiver;
fn acknowledge(f: &fixture::Fixture, rx: &Receiver<EngineCommand>) -> Vec<Action> {
    let actions = apply(f, rx);
    let next = inputforge_app::bridge::prepare(&f.state.read(), Some(&fixture::key()));
    let mut config = f.config.borrow().unwrap();
    if *config.peek() != next.config {
        config.set(next.config);
    }
    actions
}
fn apply(f: &fixture::Fixture, rx: &Receiver<EngineCommand>) -> Vec<Action> {
    let EngineCommand::SetMapping {
        input,
        mode,
        name,
        actions,
    } = rx.try_recv().unwrap()
    else {
        panic!("expected mapping");
    };
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("expected barrier");
    };
    let mut state = f.state.write();
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&input, &mode, name, actions.clone());
    state.mapping_edit_barrier = Some((owner, serial));
    drop(state);
    actions
}
fn plot(dom: &VirtualDom) -> String {
    let html = dioxus_ssr::render(dom);
    let fragment = scraper::Html::parse_fragment(&html);
    let selector = scraper::Selector::parse(".if-curve__plot-frame svg").unwrap();
    fragment.select(&selector).next().unwrap().html()
}
fn symmetric(dom: &VirtualDom) -> bool {
    let html = dioxus_ssr::render(dom);
    let fragment = scraper::Html::parse_fragment(&html);
    let selector = scraper::Selector::parse("input.if-switch__input").unwrap();
    fragment
        .select(&selector)
        .next()
        .unwrap()
        .value()
        .attr("checked")
        .is_some()
}
#[test]
fn rendered_curve_keyboard_edit_undo_redo_refreshes_plot() {
    runtime(async {
        let (mut dom, f, _doc, rx) = fixture(curve());
        settle(&mut dom).await;
        dom::key(&dom, "response curve", Key::End, Code::End);
        settle(&mut dom).await;
        let before = plot(&dom);
        dom::key(&dom, "response curve", Key::ArrowUp, Code::ArrowUp);
        settle(&mut dom).await;
        let edited = plot(&dom);
        assert_ne!(edited, before);
        acknowledge(&f, &rx);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().undo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), before, "undo must refresh the mounted plot");
        assert_eq!(acknowledge(&f, &rx), vec![curve()]);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().redo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), edited, "redo must refresh the mounted plot");
        acknowledge(&f, &rx);
        settle(&mut dom).await;
        assert_eq!(plot(&dom), edited);
    });
}

#[test]
fn rendered_curve_symmetric_switch_undo_redo_refreshes_plot_and_checkbox() {
    runtime(async {
        let (mut dom, f, _doc, rx) = fixture(curve());
        settle(&mut dom).await;
        let before = plot(&dom);
        dom::send(
            &dom,
            "if-switch__input",
            "change",
            PlatformEventData::new(Box::new(SerializedFormData::new("true".into(), vec![]))),
        );
        settle(&mut dom).await;
        let edited = plot(&dom);
        assert_ne!(edited, before);
        acknowledge(&f, &rx);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().undo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), before);
        assert!(!symmetric(&dom));
        assert_eq!(acknowledge(&f, &rx), vec![curve()]);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().redo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), edited);
        assert!(symmetric(&dom));
        let restored = acknowledge(&f, &rx);
        assert!(matches!(
            &restored[0],
            Action::ResponseCurve {
                curve: ResponseCurve::PiecewiseLinear {
                    symmetric: true,
                    ..
                }
            }
        ));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), edited);
    });
}
#[test]
fn rendered_curve_completed_drag_undo_redo_refreshes_plot() {
    runtime(async {
        let (mut dom, f, doc, rx) = fixture(curve());
        dom::send(
            &dom,
            "response curve",
            "mounted",
            PlatformEventData::new(Box::new(MountedData::new(()))),
        );
        settle(&mut dom).await;
        let before = plot(&dom);
        doc.pointer(0, "down", 195.0, 52.0);
        doc.pointer(0, "move", 195.0, 80.0);
        doc.pointer(0, "up", 195.0, 80.0);
        settle(&mut dom).await;
        let edited = plot(&dom);
        assert_ne!(edited, before);
        acknowledge(&f, &rx);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().undo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), before);
        assert_eq!(acknowledge(&f, &rx), vec![curve()]);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().redo.call(fixture::key()));
        settle(&mut dom).await;
        assert_eq!(plot(&dom), edited);
    });
}

#[test]
fn rendered_deadzone_numeric_edit_undo_redo_refreshes_field() {
    runtime(async {
        let initial = Action::Deadzone {
            config: DeadzoneConfig::default(),
        };
        let (mut dom, f, _doc, rx) = fixture(initial.clone());
        settle(&mut dom).await;
        dom::send(
            &dom,
            "dz-low",
            "input",
            PlatformEventData::new(Box::new(SerializedFormData::new("-0.8".into(), vec![]))),
        );
        dom::key(&dom, "dz-low", Key::Enter, Code::Enter);
        settle(&mut dom).await;
        let edited = acknowledge(&f, &rx);
        assert_ne!(edited, vec![initial.clone()]);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().undo.call(fixture::key()));
        settle(&mut dom).await;
        assert!(dioxus_ssr::render(&dom).contains("value=\"-1.00\""));
        assert_eq!(acknowledge(&f, &rx), vec![initial]);
        settle(&mut dom).await;
        assert!(f.bindings.borrow().unwrap().redo.call(fixture::key()));
        settle(&mut dom).await;
        assert!(dioxus_ssr::render(&dom).contains("value=\"-0.80\""));
        assert_eq!(acknowledge(&f, &rx), edited);
    });
}

#[test]
fn rendered_queued_curve_undo_restores_when_engine_frame_returns_to_same_value() {
    runtime(async {
        let (mut dom, f, _doc, rx) = fixture(curve());
        settle(&mut dom).await;
        dom::key(&dom, "response curve", Key::End, Code::End);
        settle(&mut dom).await;
        let before = plot(&dom);
        dom::key(&dom, "response curve", Key::ArrowUp, Code::ArrowUp);
        settle(&mut dom).await;
        assert_ne!(plot(&dom), before);
        assert!(f.bindings.borrow().unwrap().undo.call(fixture::key()));
        apply(&f, &rx);
        assert_eq!(acknowledge(&f, &rx), vec![curve()]);
        settle(&mut dom).await;
        assert_eq!(plot(&dom), before);
        assert_eq!(
            f.editor.borrow().unwrap().undo_log.peek().stacks[&fixture::key()].undo,
            []
        );
    });
}
