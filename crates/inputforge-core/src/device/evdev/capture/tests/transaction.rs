use super::fixtures::{capture, fail, id, released, world};
use crate::device::evdev::{Access, Class, IdentityQuality};

#[test]
fn every_acquisition_stage_rolls_back_every_position() {
    for operation in ["open read-only", "verify event node", "grab"] {
        for name in ["a", "b", "c"] {
            let fake = world();
            let mut capture = capture(&fake);
            fail(&fake, operation, name);
            let error = capture.acquire([id("c"), id("a"), id("b")]).unwrap_err();
            assert_eq!(error.operation(), operation);
            assert_eq!(error.raw_os_error(), Some(16));
            assert!(capture.captured().is_empty());
            released(&fake);
        }
    }
}

#[test]
fn selection_is_deduplicated_and_active_acquisition_preserves_ownership() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("b"), id("a"), id("b")]).unwrap();
    assert_eq!(capture.captured(), &[id("a"), id("b")]);
    assert!(capture.acquire([id("c")]).is_err());
    assert_eq!(fake.borrow().grabbed.len(), 2);
    capture.release().unwrap();
    capture.release().unwrap();
    released(&fake);
}

#[test]
fn invalid_selections_never_open_capture_handles() {
    for case in 0..6 {
        let fake = world();
        match case {
            0 => fake.borrow_mut().devices[0].classification.kind = Class::Excluded,
            1 => fake.borrow_mut().devices[0].classification.kind = Class::Ambiguous,
            2 => fake.borrow_mut().devices[0].identity.id = None,
            3 => fake.borrow_mut().devices[0].identity.quality = IdentityQuality::Ambiguous,
            4 => fake.borrow_mut().devices[0].access = Access::Failed,
            _ => {
                let duplicate = fake.borrow().devices[0].clone();
                fake.borrow_mut().devices.push(duplicate);
            }
        }
        let mut capture = capture(&fake);
        assert!(capture.acquire([id("a")]).is_err());
        released(&fake);
        assert!(
            !fake
                .borrow()
                .calls
                .iter()
                .any(|call| call.starts_with("open read-only:"))
        );
    }
    let fake = world();
    let mut capture = capture(&fake);
    assert!(capture.acquire([]).is_err());
    assert!(capture.acquire([id("missing")]).is_err());
    released(&fake);
}

#[test]
fn rollback_preserves_primary_error_and_continues_after_ungrab_failure() {
    let fake = world();
    let mut capture = capture(&fake);
    fail(&fake, "grab", "c");
    fail(&fake, "ungrab", "a");
    let error = capture.acquire([id("a"), id("b"), id("c")]).unwrap_err();
    assert_eq!(error.operation(), "grab");
    assert_eq!(error.cleanup_failures().len(), 1);
    assert!(fake.borrow().calls.contains(&"ungrab:b".into()));
    released(&fake);
    fake.borrow_mut().failures.clear();
    capture.acquire([id("a")]).unwrap();
    capture.release().unwrap();
    released(&fake);
}

#[test]
fn explicit_release_closes_all_handles_even_if_ungrab_fails() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a"), id("b")]).unwrap();
    fail(&fake, "ungrab", "a");
    let error = capture.release().unwrap_err();
    assert_eq!(error.operation(), "ungrab");
    assert!(fake.borrow().calls.contains(&"ungrab:b".into()));
    released(&fake);
    capture.release().unwrap();
}

#[test]
fn drop_and_unwinding_close_owned_handles() {
    let fake = world();
    let mut owner = capture(&fake);
    owner.acquire([id("a"), id("b")]).unwrap();
    drop(owner);
    released(&fake);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut capture = capture(&fake);
        capture.acquire([id("a")]).unwrap();
        panic!("injected caller panic");
    }));
    assert!(result.is_err());
    released(&fake);
}

#[test]
fn final_inventory_change_rolls_back_before_publishing_capture() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().replace_on_scan = Some((3, vec![]));
    assert!(capture.acquire([id("a"), id("b")]).is_err());
    assert!(fake.borrow().calls.contains(&"grab:b".into()));
    released(&fake);
    assert!(capture.captured().is_empty());
}

#[test]
fn foreign_identity_is_rejected_even_if_inventory_contains_it() {
    let fake = world();
    let foreign = crate::types::DeviceId("sdl:controller".into());
    fake.borrow_mut().devices[0].identity.id = Some(foreign.clone());
    let mut capture = capture(&fake);
    assert!(capture.acquire([foreign]).is_err());
    assert!(fake.borrow().calls.is_empty());
    released(&fake);
}

#[test]
fn final_descriptor_failure_rolls_back_all_grabs_and_can_be_retried() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().dead.insert("b".into());
    let error = capture.acquire([id("a"), id("b")]).unwrap_err();
    assert_eq!(error.operation(), "poll capture descriptor");
    assert!(fake.borrow().calls.contains(&"grab:b".into()));
    released(&fake);
    fake.borrow_mut().dead.clear();
    capture.acquire([id("a"), id("b")]).unwrap();
}

#[test]
fn drop_closes_all_handles_after_multiple_ungrab_errors() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a"), id("b")]).unwrap();
    fail(&fake, "ungrab", "a");
    fail(&fake, "ungrab", "b");
    drop(capture);
    released(&fake);
    for name in ["a", "b"] {
        assert!(fake.borrow().calls.contains(&format!("ungrab:{name}")));
    }
}
