use super::super::fixtures::{capture, fail, id, released};
use super::fixtures::{event, queue, ready, report, setup};

#[test]
fn no_selection_is_a_reusable_state_error_and_release_discards_streams() {
    let (fake, mut owner) = setup(&["a"]);
    ready(&fake, &mut owner);
    owner.release().unwrap();
    assert!(owner.stream_state(&id("a")).is_none());
    let calls = fake.borrow().calls.len();
    owner.poll_stream(&mut Vec::new()).unwrap_err();
    assert_eq!(calls, fake.borrow().calls.len());
    owner.acquire([id("a")]).unwrap();
    assert!(owner.stream_state(&id("a")).is_none());
    ready(&fake, &mut owner);
    drop(owner);
    released(&fake);
}

#[test]
fn query_failure_preserves_primary_and_all_cleanup_failures() {
    let (fake, mut owner) = setup(&["a", "b"]);
    fail(&fake, "query stream state", "a");
    fail(&fake, "ungrab", "a");
    fail(&fake, "ungrab", "b");
    let error = owner.poll_stream(&mut Vec::new()).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(16));
    assert_eq!(error.device_id(), Some(&id("a")));
    assert_eq!(error.cleanup_failures().len(), 2);
    released(&fake);
    assert!(owner.acquire([id("a")]).is_err());
}

#[test]
fn incomplete_snapshot_is_fatal_and_never_published() {
    let (fake, mut owner) = setup(&["a"]);
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .state
        .axes
        .remove(&0x10);
    let mut out = Vec::new();
    owner.poll_stream(&mut out).unwrap_err();
    assert!(out.is_empty());
    released(&fake);
}

#[test]
fn selected_disconnect_revocation_and_node_replacement_release_everything() {
    for failure in 0..3 {
        let (fake, mut owner) = setup(&["a", "b"]);
        ready(&fake, &mut owner);
        let mut world = fake.borrow_mut();
        match failure {
            0 => {
                world.devices.remove(0);
                world.pending = 1;
            }
            1 => {
                world.dead.insert("a".into());
            }
            _ => {
                world.generation += 1;
            }
        }
        drop(world);
        owner.poll_stream(&mut Vec::new()).unwrap_err();
        assert!(owner.stream_state(&id("b")).is_none());
        released(&fake);
        fake.borrow_mut().dead.clear();
        assert!(owner.acquire([id("b")]).is_err());
        let mut replacement = capture(&fake);
        assert!(replacement.captured().is_empty());
        replacement.acquire([id("b")]).unwrap();
        ready(&fake, &mut replacement);
    }
}

#[test]
fn unselected_disconnect_does_not_interrupt_streaming() {
    let (fake, mut owner) = setup(&["a"]);
    ready(&fake, &mut owner);
    fake.borrow_mut().devices.remove(2);
    fake.borrow_mut().pending = 1;
    queue(&fake, "a", vec![event(1, 0x120, 1), report()]);
    assert!(owner.poll_stream(&mut Vec::new()).unwrap().ready);
    assert!(owner.stream_state(&id("a")).unwrap().keys[&0x120]);
}
