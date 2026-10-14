use super::support::{click, mount};
use crate::frame::view_state::MainSurface;
use dioxus::prelude::*;
#[test]
fn successful_apply_requests_snapshot_once_and_navigates_without_claiming_persistence() {
    let (mut dom, f, rx) = mount();
    click(&dom, "Apply 13 mappings");
    dom.render_immediate_to_vec();
    assert_eq!(
        *f.view.borrow().unwrap().main_surface.peek(),
        MainSurface::Mappings
    );
    let inputforge_core::engine::EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label,
    } = rx.try_recv().unwrap()
    else {
        panic!("batch expected")
    };
    assert_eq!(entries.len(), 13);
    assert_eq!(snapshot_label, "FlightStick · Device 1");
    assert!(matches!(
        rx.try_recv().unwrap(),
        inputforge_core::engine::EngineCommand::MappingEditBarrier { .. }
    ));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    let messages = format!("{:?}", f.toasts.borrow().unwrap().peek());
    assert!(messages.contains("Requested 13 mappings"));
    assert!(!messages.contains("Created"));
    assert_eq!(
        f.state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        0
    );
}
