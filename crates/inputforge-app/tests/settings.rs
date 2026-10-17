//! Settings regression cases through real app bindings.
#[path = "settings/fixture.rs"]
mod fixture;
use dioxus::prelude::*;
use inputforge_gui_dx::settings::Intent;
#[test]
fn invalid_and_unchanged_max_never_dispatch_and_valid_bounds_work() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        for text in ["0", "101", "", "abc"] {
            assert!(!f.send(Intent::Max(text.into())));
            assert!(f.view.peek().error.is_some());
        }
        assert!(f.send(Intent::Max("10".into())));
        f.rx.try_recv().unwrap_err();
        for text in ["1", "100"] {
            assert!(f.send(Intent::Max(text.into())));
            f.rx.try_recv().unwrap();
        }
    });
}
#[test]
fn failed_delivery_retains_prune_and_values_with_feedback() {
    let (dom, f) = fixture::fixture(false);
    dom.in_scope(ScopeId::ROOT, || {
        f.state.write().active_snapshot_rows = fixture::rows(5);
        assert!(f.send(Intent::Max("2".into())));
        assert_eq!(f.view.peek().prune.as_ref().unwrap().will_remove, 3);
        assert!(!f.send(Intent::Confirm));
        assert!(f.view.peek().prune.is_some());
        assert!(f.view.peek().error.is_some());
        assert!(!f.send(Intent::Skip));
        assert!(f.view.peek().values.snapshot.skip_if_unchanged);
        assert!(f.send(Intent::Cancel));
        assert!(f.view.peek().prune.is_none());
    });
}
#[test]
fn confirmation_rejects_changed_profile_path_count_or_config() {
    for mutation in 0..4 {
        let (dom, f) = fixture::fixture(true);
        dom.in_scope(ScopeId::ROOT, || {
            f.state.write().active_snapshot_rows = fixture::rows(5);
            assert!(f.send(Intent::Max("2".into())));
            match mutation {
                0 => f.state.write().active_profile = Some(fixture::profile()),
                1 => f.state.write().profile_path = Some("/other.toml".into()),
                2 => {
                    f.state.write().active_snapshot_rows.pop();
                }
                _ => {
                    f.state.write().snapshot_config.skip_if_unchanged = false;
                }
            }
            assert!(!f.send(Intent::Confirm));
            assert!(f.view.peek().prune.is_some());
            f.rx.try_recv().unwrap_err();
        });
    }
}
#[test]
fn rapid_switches_and_cross_field_commits_keep_pending_values_until_ack() {
    use inputforge_core::engine::{EngineCommand, SettingsChange};
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        for intent in [
            Intent::Skip,
            Intent::Skip,
            Intent::Autostart,
            Intent::Autostart,
            Intent::Minimized,
            Intent::Minimized,
            Intent::Max("25".into()),
            Intent::Skip,
            Intent::DoubleTap(350),
            Intent::LongPress(800),
        ] {
            assert!(f.send(intent));
            f.c.refresh();
        }
        let requests: Vec<_> =
            f.rx.try_iter()
                .map(|c| match c {
                    EngineCommand::Settings(r) => r,
                    other => panic!("unexpected {other:?}"),
                })
                .collect();
        assert_eq!(requests.len(), 10);
        assert_eq!(requests[0].change, SettingsChange::Skip(false));
        assert_eq!(requests[1].change, SettingsChange::Skip(true));
        assert_eq!(requests[2].change, SettingsChange::Autostart(true));
        assert_eq!(requests[3].change, SettingsChange::Autostart(false));
        assert_eq!(requests[4].change, SettingsChange::Minimized(true));
        assert_eq!(requests[5].change, SettingsChange::Minimized(false));
        assert_eq!(f.view.peek().values.snapshot.max_count, 25);
        assert!(!f.view.peek().values.snapshot.skip_if_unchanged);
        assert_eq!(f.view.peek().values.default_double_tap_threshold_ms, 350);
        assert_eq!(f.view.peek().values.default_long_press_threshold_ms, 800);
        let last = requests.last().unwrap();
        f.state.write().settings_ack = Some((last.owner, last.serial));
        f.c.refresh();
        assert_eq!(f.view.peek().values.snapshot.max_count, 10);
        assert!(f.view.peek().error.is_some());
    });
}
#[test]
fn gesture_invalid_noop_and_dropped_owner_do_not_deliver() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        for intent in [
            Intent::DoubleTap(0),
            Intent::DoubleTap(10001),
            Intent::LongPress(0),
            Intent::LongPress(10001),
        ] {
            assert!(!f.send(intent));
        }
        let values = f.view.peek().values.clone();
        assert!(f.send(Intent::DoubleTap(
            usize::try_from(values.default_double_tap_threshold_ms).unwrap()
        )));
        assert!(f.send(Intent::LongPress(
            usize::try_from(values.default_long_press_threshold_ms).unwrap()
        )));
        f.rx.try_recv().unwrap_err();
        f.bindings.unmount.call(f.owner);
        assert!(!f.send(Intent::Skip));
        f.rx.try_recv().unwrap_err();
    });
}
#[test]
fn old_mount_ack_cannot_acknowledge_new_owner_request() {
    use inputforge_core::engine::EngineCommand;
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        assert!(f.send(Intent::Skip));
        let EngineCommand::Settings(old) = f.rx.try_recv().unwrap() else {
            panic!("settings request expected")
        };
        let next = Signal::new(inputforge_gui_dx::settings::View::default());
        let owner = f.bindings.mount.call(next);
        let workflow = next.peek().workflow;
        assert!(f.bindings.dispatch.call((owner, workflow, Intent::Skip)));
        f.state.write().settings_ack = Some((old.owner, old.serial));
        f.c.refresh();
        assert!(
            !next.peek().values.snapshot.skip_if_unchanged,
            "old acknowledgement replaced the new pending choice"
        );
    });
}
#[test]
fn stale_confirmation_callback_cannot_confirm_a_reopened_workflow() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        f.state.write().active_snapshot_rows = fixture::rows(5);
        assert!(f.send(Intent::Max("2".into())));
        let old = f.view.peek().workflow;
        assert!(f.send(Intent::Cancel));
        assert!(f.send(Intent::Max("1".into())));
        assert!(!f.bindings.dispatch.call((f.owner, old, Intent::Confirm)));
        assert!(f.view.peek().prune.is_some());
        f.rx.try_recv().unwrap_err();
    });
}
