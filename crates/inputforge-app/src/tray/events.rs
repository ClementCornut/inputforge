//! Native event dispatch independent of hidden editor rendering.
use super::action::{TrayAction, TrayMenuIds};
#[cfg(target_os = "windows")]
use super::windows;
use dioxus::prelude::*;
use muda::MenuEvent;
use std::cell::RefCell;

/// Dispatch on native events, independently of `WebView` render acknowledgements.
pub(crate) fn install_native_handler(
    ids: TrayMenuIds,
    mut on_action: impl FnMut(TrayAction) + 'static,
    mut on_tick: impl FnMut() + 'static,
) {
    let dispatch = move || {
        on_tick();
        // Poll muda's receiver so early clicks and remounts cannot strand a
        // process-global OnceLock callback on an obsolete Dioxus task.
        for event in MenuEvent::receiver().try_iter() {
            if let Some(action) = TrayAction::from_event(&event, &ids) {
                on_action(action);
            }
        }
    };
    install_tick(dispatch);
}

pub(crate) fn install_tick(mut dispatch: impl FnMut() + 'static) {
    #[cfg(target_os = "linux")]
    {
        let runtime = dioxus_core::Runtime::current();
        let scope = dioxus_core::current_scope_id();
        dioxus_core::use_hook_with_cleanup(
            move || {
                std::rc::Rc::new(RefCell::new(Some(glib::timeout_add_local(
                    std::time::Duration::from_millis(250),
                    move || {
                        runtime.in_scope(scope, &mut dispatch);
                        glib::ControlFlow::Continue
                    },
                ))))
            },
            |source| {
                if let Some(source) = source.borrow_mut().take() {
                    source.remove();
                }
            },
        );
    }
    #[cfg(not(target_os = "linux"))]
    {
        dioxus_core::use_hook_with_cleanup(
            || {
                let native = std::sync::Arc::downgrade(&dioxus::desktop::window().window);
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        let Some(native) = native.upgrade() else {
                            break;
                        };
                        #[cfg(target_os = "windows")]
                        if !windows::wake(&native) {
                            break;
                        }
                        #[cfg(not(target_os = "windows"))]
                        native.request_redraw();
                    }
                })
                .abort_handle()
            },
            |timer| timer.abort(),
        );
        dioxus::desktop::use_wry_event_handler(move |_, _| dispatch());
    }
}
