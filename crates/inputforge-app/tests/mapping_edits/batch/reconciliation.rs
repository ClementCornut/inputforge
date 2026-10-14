use super::*;
use dioxus::prelude::WritableExt;
#[test]
fn apply_revalidates_profile_source_target_and_mode_before_rerender() {
    for change in 0..6 {
        let (_dom, f, rx) = mount();
        let request = f.request(Intent::Apply);
        match change {
            0 => f.state.write().active_profile = None,
            1 => f.state.write().devices[0].connected = false,
            2 => f.state.write().virtual_devices.clear(),
            3 => f.state.write().virtual_devices[0].axes.reverse(),
            4 => f.state.write().session.offline = true,
            _ => f.mode.borrow().unwrap().set("Other".into()),
        }
        f.batch.borrow().unwrap().dispatch.call(request);
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty,
            "change {change}"
        );
        assert_eq!(f.completed.get(), 0);
        assert!(f.model().error.is_some());
    }
}
#[test]
fn profile_identity_change_resets_draft_even_when_name_matches() {
    let (_dom, f, rx) = mount();
    f.act(Intent::Exclude(RowKind::Axis));
    let request = f.request(Intent::Apply);
    let mut replacement = f.state.read().active_profile.as_ref().unwrap().clone();
    replacement = inputforge_core::profile::Profile::new(
        replacement.name().to_owned(),
        vec![],
        replacement.modes().clone(),
        vec![],
        vec![],
        "Default".into(),
    );
    f.state.write().active_profile = Some(replacement);
    f.batch.borrow().unwrap().dispatch.call(request);
    assert!(f.model().error.is_some());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    f.refresh();
    assert!(!f.model().dirty);
    assert_eq!(f.model().counts.create, 5);
}
#[test]
fn refresh_preserves_valid_overrides_and_rejects_obsolete_row_callbacks() {
    let (_dom, f, rx) = mount();
    f.act(Intent::RowTarget {
        kind: RowKind::Axis,
        index: 0,
        value: "axis:Y".into(),
    });
    f.refresh();
    assert_eq!(
        f.model().rows[0].target.as_ref().unwrap().output,
        OutputId::Axis { id: VJoyAxis::Y }
    );
    let old = f.request(Intent::Exclude(RowKind::Axis));
    let mut other = f.state.read().devices[0].clone();
    other.info.id = inputforge_core::types::DeviceId("other".into());
    f.state.write().devices.push(other);
    f.act(Intent::Source("other".into()));
    f.batch.borrow().unwrap().dispatch.call(old);
    assert_eq!(f.model().counts.excluded, 0);
    assert!(f.model().error.is_some());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn native_layout_and_tombstones_revalidate_without_substituting_sparse_controls() {
    use inputforge_core::profile::controllers::{AxisBinding, DeviceBinding};
    use inputforge_core::types::{AxisPolarity, DeviceId, InputId};
    let (_dom, f, rx) = mount();
    let b = DeviceBinding {
        device: DeviceId("batch".into()),
        observed_layout: None,
        unavailable: vec![InputId::Axis { index: 0 }],
        axes: vec![
            AxisBinding {
                code: 2,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Bipolar,
            },
            AxisBinding {
                code: 24,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Bipolar,
            },
        ],
        buttons: vec![304, 310],
        hats: vec![2],
    };
    f.state.write().session.bindings.push(b);
    f.refresh();
    assert_eq!(f.model().rows[0].input, address(1));
    assert_eq!(
        f.model().rows[0].target.as_ref().unwrap().output,
        OutputId::Axis { id: VJoyAxis::Y }
    );
    let request = f.request(Intent::Apply);
    f.state.write().session.bindings[0].axes[1].code = 25;
    f.batch.borrow().unwrap().dispatch.call(request);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    f.refresh();
    f.act(Intent::Apply);
    let entries = commands(&f.state, &rx);
    assert!(!entries.iter().any(|e| e.input == address(0)));
}
#[test]
fn missing_source_falls_back_and_target_capability_changes_preserve_valid_choices() {
    let (_dom, f, _) = mount();
    f.act(Intent::RowTarget {
        kind: RowKind::Axis,
        index: 0,
        value: "axis:Y".into(),
    });
    f.act(Intent::Replace(RowKind::Axis, 0));
    f.state.write().virtual_devices[0].button_count = 1;
    f.refresh();
    assert!(f.model().rows[3].target.is_none());
    assert_eq!(
        f.model().rows[0].target.as_ref().unwrap().output,
        OutputId::Axis { id: VJoyAxis::Y }
    );
    assert!(f.model().rows[0].replace);
    let mut other = f.state.read().devices[0].clone();
    other.info.id = inputforge_core::types::DeviceId("fallback".into());
    f.state.write().devices.push(other);
    f.state.write().devices[0].connected = false;
    f.refresh();
    assert_eq!(f.model().source.as_ref().unwrap().0, "fallback");
    assert!(!f.model().dirty);
    f.state.write().devices.clear();
    f.refresh();
    assert_eq!(f.model().rows.len(), 0);
    assert!(!f.model().can_apply);
}
