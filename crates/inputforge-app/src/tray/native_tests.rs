//! Runs in a child process because Tao's desktop loop exits the process.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::exit,
    reason = "isolated child reports a marker and terminates stalled native loops without leaking a window"
)]
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use dioxus::desktop::{Config, WindowBuilder, WindowCloseBehaviour, window};
use dioxus::prelude::*;
use muda::{ContextMenu, Menu, MenuItem, MenuItemKindSnapshot};

use super::action::{TrayAction, TrayMenuIds};

static STAGE: AtomicUsize = AtomicUsize::new(0);
const CHILD: &str = "INPUTFORGE_NATIVE_TRAY_TEST";
const TEST: &str = "tray::native_tests::hidden_window_dispatches_show_toggle_and_quit";

#[test]
#[ignore = "opens an isolated native window; requires a desktop session"]
fn hidden_window_dispatches_show_toggle_and_quit() {
    if std::env::var_os(CHILD).is_some() {
        run_window(smoke_root, 2, true);
    } else {
        run_child(TEST);
    }
}

#[test]
#[ignore = "opens an isolated native window; requires a desktop session"]
fn hidden_window_relaunch_shows_without_tray() {
    if std::env::var_os(CHILD).is_some() {
        run_window(relaunch_root, 3, false);
    } else {
        run_child("tray::native_tests::hidden_window_relaunch_shows_without_tray");
    }
}

fn run_window(root: fn() -> Element, expected: usize, visible: bool) {
    #[cfg(target_os = "linux")]
    use dioxus::desktop::tao::platform::unix::EventLoopBuilderExtUnix;
    #[cfg(target_os = "windows")]
    use dioxus::desktop::tao::platform::windows::EventLoopBuilderExtWindows;
    use dioxus::desktop::tao::{event::Event, event_loop::EventLoopBuilder};
    let event_loop = EventLoopBuilder::with_user_event()
        .with_any_thread(true)
        .build();
    let cfg = Config::new()
        .with_event_loop(event_loop)
        .with_window(
            WindowBuilder::new()
                .with_title("InputForge tray regression")
                .with_visible(visible),
        )
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        .with_custom_event_handler(move |event, _| {
            if matches!(event, Event::LoopDestroyed) {
                assert_eq!(STAGE.load(Ordering::Acquire), expected);
                println!("NATIVE_TRAY_REGRESSION_PASSED");
            }
        });
    LaunchBuilder::desktop().with_cfg(cfg).launch(root);
}

fn run_child(test: &str) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            test,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .env("GDK_BACKEND", "wayland")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("NATIVE_TRAY_REGRESSION_PASSED"),
        "native tray regression failed\n{stdout}\n{stderr}"
    );
}

fn smoke_root() -> Element {
    let ids = use_hook(|| {
        let show = MenuItem::new("Show", true, None);
        let toggle = MenuItem::new("Toggle", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let ids = TrayMenuIds {
            show: show.id().clone(),
            toggle: toggle.id().clone(),
            quit: quit.id().clone(),
        };
        let menu = Menu::new();
        menu.append_items(&[&show, &toggle, &quit]).unwrap();
        let actions: Vec<_> = menu
            .snapshot_handle()
            .items()
            .into_iter()
            .map(|item| {
                let MenuItemKindSnapshot::MenuItem(item) = item else {
                    panic!("expected menu item");
                };
                Arc::clone(&item.activate)
            })
            .collect();
        let native = Arc::downgrade(&window().window);
        window().set_visible(false);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            actions[0]();
            wait_for(
                || native.upgrade().is_some_and(|w| w.is_visible()),
                "Show while hidden",
            );
            native.upgrade().unwrap().set_visible(false);
            STAGE.store(1, Ordering::Release);
            actions[1]();
            wait_for(|| STAGE.load(Ordering::Acquire) == 2, "Toggle while hidden");
            // Quit must work from a hidden window too.
            native.upgrade().unwrap().set_visible(false);
            actions[2]();
        });
        ids
    });
    super::events::install_native_handler(
        ids,
        |action| match action {
            TrayAction::Show => super::window::show_window(),
            TrayAction::Toggle => {
                STAGE.store(2, Ordering::Release);
            }
            TrayAction::Quit => super::window::request_quit(),
        },
        || {},
    );
    rsx! { div { "Native tray regression" } }
}

fn wait_for(mut ready: impl FnMut() -> bool, operation: &str) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !ready() {
        if Instant::now() > deadline {
            eprintln!("Timed out: {operation}");
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

struct RelaunchFixture {
    _directory: tempfile::TempDir,
    instance: Arc<crate::single_instance::Instance>,
    quit: Arc<std::sync::atomic::AtomicBool>,
}

fn relaunch_root() -> Element {
    let fixture = use_hook(|| {
        let directory = tempfile::tempdir().unwrap();
        let instance = Arc::new(
            crate::single_instance::Instance::acquire(directory.path())
                .unwrap()
                .unwrap(),
        );
        let quit = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let quit_request = Arc::clone(&quit);
        let path = directory.path().to_owned();
        let native = Arc::downgrade(&window().window);
        window().set_visible(false);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            assert!(
                crate::single_instance::Instance::acquire(&path)
                    .unwrap()
                    .is_none()
            );
            wait_for(
                || native.upgrade().is_some_and(|w| w.is_visible()),
                "relaunch while hidden without tray",
            );
            STAGE.store(3, Ordering::Release);
            quit_request.store(true, Ordering::Release);
        });
        std::rc::Rc::new(RelaunchFixture {
            _directory: directory,
            instance,
            quit,
        })
    });
    super::install(None);
    let (commands, _receiver) = std::sync::mpsc::channel();
    let handles = crate::desktop_host::DesktopHandles {
        state: Arc::new(parking_lot::RwLock::new(
            inputforge_core::state::AppState::new(),
        )),
        commands,
        initially_available: false,
        instance: Arc::clone(&fixture.instance),
    };
    let navigation = inputforge_gui_dx::EditorNavigation {
        panel: use_signal(inputforge_gui_dx::PanelSlot::default),
        calibration: use_signal(|| false),
    };
    let routing = crate::routing::bindings(handles.clone(), navigation);
    super::use_controller(handles, routing, move || {
        fixture
            .instance
            .process_pending(super::window::show_window)
            .unwrap();
        if fixture.quit.swap(false, Ordering::AcqRel) {
            super::window::request_quit();
        }
    });
    rsx! { div { "Single-instance activation regression" } }
}
