//! These fixtures use a private bus, never the user's desktop session.
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::host_available;

#[derive(Default)]
struct State {
    host: bool,
    items: Vec<String>,
}

struct Watcher(Arc<Mutex<State>>);

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        self.0.lock().unwrap().host
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.0.lock().unwrap().items.clone()
    }
}

struct Item(String);

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn id(&self) -> &str {
        &self.0
    }
}

struct Bus(Child);

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_bus() -> (Bus, String) {
    let mut bus = Bus(Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap());
    let mut address = String::new();
    BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    (bus, address.trim().to_owned())
}

#[test]
#[ignore = "requires dbus-daemon to start an isolated session bus"]
fn private_bus_tracks_registration_loss_and_panel_restart() {
    let (_bus, address) = start_bus();
    let address = address.as_str();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = zbus::connection::Builder::address(address)
            .unwrap()
            .build()
            .await
            .unwrap();
        let tray_id = format!("inputforge-{}", std::process::id());
        let check = || async {
            tokio::time::timeout(Duration::from_secs(2), host_available(&client, &tray_id))
                .await
                .unwrap()
        };
        assert!(check().await.is_err(), "no watcher must not allow hiding");
        let state = Arc::new(Mutex::new(State::default()));
        let server = zbus::connection::Builder::address(address)
            .unwrap()
            .name("org.kde.StatusNotifierWatcher")
            .unwrap()
            .serve_at("/StatusNotifierWatcher", Watcher(Arc::clone(&state)))
            .unwrap()
            .serve_at("/StatusNotifierItem", Item(tray_id.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        assert!(!check().await.unwrap());
        server
            .request_name("org.kde.StatusNotifierHost-123")
            .await
            .unwrap();
        state.lock().unwrap().host = true;
        assert!(
            !check().await.unwrap(),
            "host without our registered item is insufficient"
        );
        // Exercise sandboxed KSNI's unique-name registration.
        state.lock().unwrap().items.push(format!(
            "{}/StatusNotifierItem",
            server.unique_name().unwrap()
        ));
        assert!(check().await.unwrap());
        state.lock().unwrap().host = false;
        assert!(
            !check().await.unwrap(),
            "host property must not remain cached"
        );
        state.lock().unwrap().host = true;
        state.lock().unwrap().items.clear();
        assert!(
            !check().await.unwrap(),
            "item removal must not remain cached"
        );
        state.lock().unwrap().items.push(format!(
            "org.kde.StatusNotifierItem-{}-1/StatusNotifierItem",
            std::process::id()
        ));
        assert!(check().await.unwrap());
        server
            .release_name("org.kde.StatusNotifierHost-123")
            .await
            .unwrap();
        assert!(
            !check().await.unwrap(),
            "a surviving watcher and item cannot replace a live host"
        );
        // A different shell owner cannot make this watcher an integrated host.
        client.request_name("org.gnome.Shell").await.unwrap();
        assert!(!check().await.unwrap());
        client.release_name("org.gnome.Shell").await.unwrap();
        // GNOME's integrated watcher/host must belong to the same shell.
        server.request_name("org.gnome.Shell").await.unwrap();
        assert!(check().await.unwrap());
        server.release_name("org.gnome.Shell").await.unwrap();
        assert!(!check().await.unwrap());
        server
            .request_name("org.kde.StatusNotifierHost-123")
            .await
            .unwrap();
        assert!(check().await.unwrap());
        server
            .release_name("org.kde.StatusNotifierWatcher")
            .await
            .unwrap();
        check().await.unwrap_err();
        server
            .request_name("org.kde.StatusNotifierWatcher")
            .await
            .unwrap();
        assert!(
            check().await.unwrap(),
            "panel restart must recover without restarting the app"
        );
    });
}
