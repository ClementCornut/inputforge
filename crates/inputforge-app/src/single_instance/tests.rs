use super::Instance;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn second_launch_notifies_primary_and_release_allows_restart() {
    let directory = tempfile::tempdir().unwrap();
    let primary = Arc::new(Instance::acquire(directory.path()).unwrap().unwrap());
    let path = directory.path().to_owned();
    let secondary = std::thread::spawn(move || Instance::acquire(&path).unwrap().is_none());
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut shown = false;
    while !secondary.is_finished() && Instant::now() < deadline {
        primary.process_pending(|| shown = true).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        secondary.join().unwrap(),
        "second launch must not own another instance"
    );
    assert!(shown, "second launch must request the existing editor");
    drop(primary);
    assert!(
        Instance::acquire(directory.path()).unwrap().is_some(),
        "ownership must release on exit"
    );
}

#[test]
fn separate_user_directories_do_not_share_ownership() {
    let first_directory = tempfile::tempdir().unwrap();
    let second_directory = tempfile::tempdir().unwrap();
    let first = Instance::acquire(first_directory.path()).unwrap().unwrap();
    let second = Instance::acquire(second_directory.path()).unwrap().unwrap();
    drop((first, second));
}

#[test]
fn invalid_requests_do_not_show_the_editor() {
    let directory = tempfile::tempdir().unwrap();
    let primary = Instance::acquire(directory.path()).unwrap().unwrap();
    let client = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    client
        .send_to(b"SHOW wrong-token", primary.socket.local_addr().unwrap())
        .unwrap();
    let mut shown = false;
    primary.process_pending(|| shown = true).unwrap();
    assert!(
        !shown,
        "unauthenticated requests must not activate the editor"
    );
    client
        .send_to(
            primary.request.as_bytes(),
            primary.socket.local_addr().unwrap(),
        )
        .unwrap();
    primary.process_pending(|| shown = true).unwrap();
    assert!(shown, "authenticated requests must activate the editor");
}

#[test]
fn endpoint_cannot_redirect_requests_off_loopback() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = directory.path().join(super::ENDPOINT);
    std::fs::write(
        &endpoint,
        format!("192.0.2.1:12345\n{}", uuid::Uuid::new_v4()),
    )
    .unwrap();
    assert_eq!(
        super::request_show(&endpoint).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn child_process_notification_and_crash_recovery() {
    use std::process::{Command, Stdio};
    let directory = tempfile::tempdir().unwrap();
    let mut owner = Command::new(std::env::current_exe().unwrap())
        .args([
            "single_instance::tests::process_fixture",
            "--exact",
            "--ignored",
        ])
        .env("INPUTFORGE_INSTANCE_TEST_DIR", directory.path())
        .env("INPUTFORGE_INSTANCE_TEST_ROLE", "owner")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !directory.path().join("ready").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "single_instance::tests::process_fixture",
            "--exact",
            "--ignored",
        ])
        .env("INPUTFORGE_INSTANCE_TEST_DIR", directory.path())
        .env("INPUTFORGE_INSTANCE_TEST_ROLE", "notify")
        .output()
        .unwrap();
    // Kill instead of orderly Drop to exercise kernel lock release after a crash.
    let _ = owner.kill();
    owner.wait().unwrap();
    assert!(
        result.status.success(),
        "secondary process failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        directory.path().join("shown").exists(),
        "primary must receive Show from another process"
    );
    assert!(
        Instance::acquire(directory.path()).unwrap().is_some(),
        "stale endpoint must not block recovery after a crash"
    );
}

#[test]
#[ignore = "child fixture; no work without an isolated test directory"]
fn process_fixture() {
    let Some(directory) = std::env::var_os("INPUTFORGE_INSTANCE_TEST_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let instance = Instance::acquire(&directory).unwrap();
    if std::env::var("INPUTFORGE_INSTANCE_TEST_ROLE").unwrap() == "notify" {
        assert!(
            instance.is_none(),
            "secondary process must not become another owner"
        );
        return;
    }
    let instance = instance.unwrap();
    std::fs::write(directory.join("ready"), []).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        instance
            .process_pending(|| std::fs::write(directory.join("shown"), []).unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn unresponsive_owner_does_not_allow_a_duplicate_instance() {
    let directory = tempfile::tempdir().unwrap();
    let primary = Instance::acquire(directory.path()).unwrap().unwrap();
    let Err(error) = Instance::acquire(directory.path()) else {
        panic!("an unresponsive owner must not allow another instance");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    let deadline = Instant::now() + Duration::from_secs(6);
    let path = directory.path().to_owned();
    let secondary = std::thread::spawn(move || Instance::acquire(&path).unwrap().is_none());
    while !secondary.is_finished() && Instant::now() < deadline {
        primary.process_pending(|| {}).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        secondary.join().unwrap(),
        "original owner must still be the only instance after a timeout"
    );
}
