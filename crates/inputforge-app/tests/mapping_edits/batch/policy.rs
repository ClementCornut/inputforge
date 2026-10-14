use super::*;
#[test]
fn defaults_preserve_axis_order_and_one_based_button_hat_outputs() {
    let (_dom, f, _) = mount();
    let v = f.model();
    assert_eq!(v.rows.len(), 5);
    assert_eq!(
        v.rows[0].target.as_ref().unwrap().output,
        OutputId::Axis { id: VJoyAxis::X }
    );
    assert_eq!(
        v.rows[1].target.as_ref().unwrap().output,
        OutputId::Axis { id: VJoyAxis::Y }
    );
    assert_eq!(
        v.rows[2].target.as_ref().unwrap().output,
        OutputId::Button { id: 1 }
    );
    assert_eq!(
        v.rows[4].target.as_ref().unwrap().output,
        OutputId::Hat { id: 1 }
    );
    assert_eq!(v.counts.create, 5);
    assert!(!v.dirty);
}
#[test]
fn invalid_row_values_never_become_exclusions_or_out_of_range_outputs() {
    for value in ["axis:Z", "button:1", "axis:bogus", "garbage"] {
        let (_dom, f, rx) = mount();
        f.act(Intent::RowTarget {
            kind: RowKind::Axis,
            index: 0,
            value: value.into(),
        });
        assert!(f.model().error.is_some());
        assert_eq!(
            f.model().rows[0].target.as_ref().unwrap().output,
            OutputId::Axis { id: VJoyAxis::X }
        );
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
}
#[test]
fn overflow_include_exclude_and_reset_use_current_targets() {
    let (_dom, f, _) = mount();
    f.state.write().virtual_devices[0].axes.truncate(1);
    f.refresh();
    assert!(f.model().rows[1].target.is_none());
    f.act(Intent::Exclude(RowKind::Axis));
    assert_eq!(f.model().counts.excluded, 2);
    f.act(Intent::Include(RowKind::Axis));
    assert!(f.model().rows[0].target.is_some());
    assert!(f.model().rows[1].target.is_none());
    f.act(Intent::Reset);
    assert!(!f.model().dirty);
}
#[test]
fn collisions_fan_out_per_mode_but_exclusions_count_per_row() {
    let (_dom, f, rx) = mount();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(
            &address(0),
            "Default",
            Some("Existing".into()),
            vec![Action::Invert],
        );
    f.act(Intent::AllModes(true));
    assert_eq!(f.model().counts.create, 9);
    assert_eq!(f.model().counts.skip, 1);
    f.act(Intent::ReplaceConflicts {
        kind: RowKind::Axis,
        replace: true,
    });
    assert_eq!(f.model().counts.replace, 1);
    f.act(Intent::RowTarget {
        kind: RowKind::Axis,
        index: 1,
        value: "(do not map)".into(),
    });
    assert_eq!(f.model().counts.excluded, 1);
    f.act(Intent::Apply);
    let entries = commands(&f.state, &rx);
    assert_eq!(entries.len(), 8);
    assert_eq!(entries.iter().filter(|e| e.input == address(0)).count(), 2);
    assert_eq!(entries.iter().filter(|e| e.input == address(1)).count(), 0);
}

#[test]
fn target_caps_and_overflow_cover_sliders_buttons_and_hats() {
    let (_dom, f, _) = mount();
    let mut s = f.state.write();
    s.devices[0].info.axes = 9;
    s.devices[0].info.buttons = 255;
    s.devices[0].info.hats = 2;
    s.virtual_devices[0].axes = vec![
        VJoyAxis::X,
        VJoyAxis::Y,
        VJoyAxis::Z,
        VJoyAxis::Rx,
        VJoyAxis::Ry,
        VJoyAxis::Rz,
        VJoyAxis::Slider0,
        VJoyAxis::Slider1,
    ];
    s.virtual_devices[0].button_count = 255;
    drop(s);
    f.refresh();
    let v = f.model();
    assert_eq!(
        v.rows[7].target.as_ref().unwrap().output,
        OutputId::Axis {
            id: VJoyAxis::Slider1
        }
    );
    assert!(v.rows[8].target.is_none());
    assert_eq!(
        v.rows[263].target.as_ref().unwrap().output,
        OutputId::Button { id: 255 }
    );
    assert!(v.rows[265].target.is_none());
    f.act(Intent::RowTarget {
        kind: RowKind::Button,
        index: 0,
        value: "button:0".into(),
    });
    assert!(f.model().error.is_some());
    f.act(Intent::RowTarget {
        kind: RowKind::Hat,
        index: 0,
        value: "hat:2".into(),
    });
    assert!(f.model().error.is_some());
}
#[test]
fn replacement_without_collision_is_create_and_skipping_restores_counts() {
    let (_dom, f, _) = mount();
    f.act(Intent::Replace(RowKind::Axis, 0));
    assert_eq!(f.model().counts.create, 5);
    assert_eq!(f.model().counts.replace, 0);
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&address(0), "Default", None, vec![]);
    f.refresh();
    assert_eq!(f.model().counts.replace, 1);
    f.act(Intent::ReplaceConflicts {
        kind: RowKind::Axis,
        replace: false,
    });
    assert_eq!(f.model().counts.skip, 1);
}
#[test]
fn source_target_and_mode_choices_reject_unknown_values() {
    let (_dom, f, rx) = mount();
    for intent in [
        Intent::Source("unknown".into()),
        Intent::Target("bad".into()),
        Intent::Target("2".into()),
        Intent::Mode("deleted".into()),
    ] {
        f.act(intent);
        assert!(f.model().error.is_some());
        assert_eq!(f.model().rows[0].input, address(0));
        assert_eq!(f.model().target, Some(1));
        assert_eq!(f.model().mode, "Default");
    }
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    f.act(Intent::Mode("Other".into()));
    assert!(f.model().error.is_none());
    f.act(Intent::Apply);
    assert!(commands(&f.state, &rx).iter().all(|e| e.mode == "Other"));
}
