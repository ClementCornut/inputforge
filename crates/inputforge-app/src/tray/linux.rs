//! Linux tray availability, observed over the session D-Bus.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use zbus::{Connection, Proxy, proxy::CacheProperties};

const INTERVAL: Duration = Duration::from_millis(500);

pub(super) struct HostMonitor {
    pub available: Arc<AtomicBool>,
    stop: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}

impl HostMonitor {
    pub(super) fn new(tray_id: String) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let available = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&available);
        let (stop, shutdown) = mpsc::channel();
        let (ready, initial) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("tray-host".into())
            .spawn(move || {
                let mut connection = None;
                loop {
                    let result = runtime.block_on(async {
                        tokio::time::timeout(INTERVAL, probe(&mut connection, &tray_id)).await
                    });
                    let current = if let Ok(Ok(current)) = result {
                        current
                    } else {
                        connection = None;
                        false
                    };
                    observed.store(current, Ordering::Release);
                    let _ = ready.try_send(());
                    if shutdown.recv_timeout(INTERVAL) != Err(mpsc::RecvTimeoutError::Timeout) {
                        break;
                    }
                }
            })?;
        // Startup stays bounded even when the session bus is unresponsive.
        let _ = initial.recv_timeout(Duration::from_secs(1));
        Ok(Self {
            available,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for HostMonitor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

async fn probe(connection: &mut Option<Connection>, tray_id: &str) -> zbus::Result<bool> {
    if connection.is_none() {
        *connection = Some(Connection::session().await?);
    }
    let Some(connection) = connection.as_ref() else {
        return Ok(false);
    };
    host_available(connection, tray_id).await
}

async fn host_available(connection: &Connection, tray_id: &str) -> zbus::Result<bool> {
    // Never use cached values: a panel restart can invalidate registration.
    let watcher = zbus::proxy::Builder::<Proxy<'_>>::new(connection)
        .destination("org.kde.StatusNotifierWatcher")?
        .path("/StatusNotifierWatcher")?
        .interface("org.kde.StatusNotifierWatcher")?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let host: bool = watcher
        .get_property("IsStatusNotifierHostRegistered")
        .await?;
    if !host || !live_host(connection).await? {
        return Ok(false);
    }
    let items: Vec<String> = watcher
        .get_property("RegisteredStatusNotifierItems")
        .await?;
    if registered(host, &items, std::process::id()) {
        return Ok(true);
    }
    // Sandboxed KSNI uses a unique bus name instead of a process-named service.
    for item in items.iter().filter(|item| item.starts_with(':')) {
        let (service, path) = item.split_once('/').map_or(
            (item.as_str(), "/StatusNotifierItem".to_owned()),
            |(service, path)| (service, format!("/{path}")),
        );
        let item = zbus::proxy::Builder::<Proxy<'_>>::new(connection)
            .destination(service)?
            .path(path)?
            .interface("org.kde.StatusNotifierItem")?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        if item
            .get_property::<String>("Id")
            .await
            .is_ok_and(|id| id == tray_id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn live_host(connection: &Connection) -> zbus::Result<bool> {
    let bus = zbus::fdo::DBusProxy::new(connection).await?;
    let names = bus.list_names().await?;
    // Plasma's watcher outlives its panel and hardcodes host=true. Its actual
    // host owns org.kde.StatusNotifierHost-<pid>; check that ownership too.
    if names.iter().any(|name| {
        name.as_str().starts_with("org.kde.StatusNotifierHost-")
            || name
                .as_str()
                .starts_with("org.freedesktop.StatusNotifierHost-")
    }) {
        return Ok(true);
    }
    // GNOME's AppIndicator extension integrates the watcher into the shell.
    if !names.iter().any(|name| name.as_str() == "org.gnome.Shell") {
        return Ok(false);
    }
    Ok(bus
        .get_name_owner("org.kde.StatusNotifierWatcher".try_into()?)
        .await?
        == bus.get_name_owner("org.gnome.Shell".try_into()?).await?)
}

fn registered(host: bool, items: &[String], pid: u32) -> bool {
    let prefix = format!("org.kde.StatusNotifierItem-{pid}-");
    host && items.iter().any(|item| item.starts_with(&prefix))
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod bus_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_item_requires_both_host_and_our_process() {
        let own = "org.kde.StatusNotifierItem-42-1/StatusNotifierItem";
        assert!(registered(true, &[own.into()], 42));
        assert!(!registered(false, &[own.into()], 42));
        assert!(!registered(true, &[], 42));
        assert!(!registered(true, &[own.into()], 4));
        assert!(!registered(
            true,
            &["org.kde.StatusNotifierItem-420-1".into()],
            42
        ));
    }
}
