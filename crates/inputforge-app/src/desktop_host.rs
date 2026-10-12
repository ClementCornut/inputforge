//! Desktop configuration and lifecycle belong to the application host.
use crate::tray::{self, AppTray, availability::WindowPolicy};
use anyhow::Result;
use dioxus::{
    desktop::{Config, LogicalSize, WindowBuilder},
    prelude::*,
};
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::{EditorNavigation, PanelSlot};
use parking_lot::RwLock;
use std::sync::{Arc, atomic::Ordering, mpsc};

#[derive(Clone)]
pub(crate) struct DesktopHandles {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub initially_available: bool,
    pub instance: Arc<crate::single_instance::Instance>,
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "shared fallible desktop launch contract"
)]
pub(crate) fn launch(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray: Option<AppTray>,
    minimized: bool,
    mut on_exit: Box<dyn FnMut()>,
    instance: Arc<crate::single_instance::Instance>,
) -> Result<()> {
    let available = tray
        .as_ref()
        .is_some_and(|tray| tray.available.load(Ordering::Acquire));
    let policy = WindowPolicy::new(available, minimized);
    tray::install(tray);
    // Construction-time visibility avoids a first-paint race on Windows.
    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0))
        .with_visible(policy.visible);
    let cfg = Config::new()
        .with_window(window)
        .with_close_behaviour(policy.close_behavior())
        .with_custom_event_handler(move |event, _| {
            if matches!(event, dioxus::desktop::tao::event::Event::LoopDestroyed) {
                // Tao exits the process; the engine must join before that happens.
                on_exit();
                tray::clear();
            }
        })
        // WebView2's native file-drop handler otherwise swallows HTML5 dragging.
        .with_disable_drag_drop_handler(true);
    #[cfg(all(debug_assertions, target_os = "windows"))]
    let cfg = {
        const WRY_DEFAULTS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";
        cfg.with_windows_browser_args(format!(
            "{WRY_DEFAULTS} --remote-debugging-port=9222 --remote-allow-origins=*"
        ))
    };
    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(DesktopHandles {
            state,
            commands,
            initially_available: available,
            instance,
        })
        .launch(desktop_root);
    Ok(())
}

fn desktop_root() -> Element {
    let handles = use_context::<DesktopHandles>();
    let navigation = EditorNavigation {
        panel: use_signal(PanelSlot::default),
        calibration: use_signal(|| false),
    };
    use_context_provider(|| navigation);
    let instance = Arc::clone(&handles.instance);
    let routing = crate::routing::bindings(handles.clone(), navigation);
    tray::use_controller(handles.clone(), routing, move || {
        if let Err(error) = instance.process_pending(tray::window::show_window) {
            tracing::warn!(%error, "failed to process editor activation request");
        }
    });
    let state = Arc::clone(&handles.state);
    let start_polling = Callback::new(move |publisher| {
        crate::bridge::spawn_polling_task(Arc::clone(&state), publisher);
    });
    let controllers =
        inputforge_app::controllers::bindings(Arc::clone(&handles.state), handles.commands.clone());
    inputforge_gui_dx::render_editor(
        handles.state,
        handles.commands,
        routing,
        controllers,
        start_polling,
    )
}
