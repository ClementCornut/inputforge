//! Application-owned notification lifetime, interaction and warning delivery.
mod preview;
mod state;
mod warnings;
use dioxus::prelude::*;
use inputforge_gui_dx::toast::{Bindings, Toast, ToastLevel, ToastQueue};
use std::{cell::RefCell, rc::Rc, time::Instant};

#[derive(Debug, Default)]
struct Mounted {
    state: state::State,
    warnings: warnings::Tail,
    disposed: bool,
}
/// Create callbacks that install an isolated notification owner per editor mount.
#[must_use]
pub fn bindings() -> Bindings {
    Bindings {
        mount: Callback::new(|visible| mount(visible, None)),
    }
}
fn mount(visible: Signal<Vec<Toast>>, deadline: Option<Signal<Option<Instant>>>) -> ToastQueue {
    let owner = Rc::new(RefCell::new(Mounted::default()));
    let push_owner = Rc::clone(&owner);
    let event_owner = Rc::clone(&owner);
    let tick_owner = Rc::clone(&owner);
    ToastQueue {
        visible,
        push: Callback::new(move |(level, message)| {
            update(&push_owner, visible, deadline, |owner| {
                owner.state.push(level, message, now());
            });
        }),
        event: Callback::new(move |(id, event)| {
            update(&event_owner, visible, deadline, |owner| {
                owner.state.event(id, &event, now());
            });
        }),
        tick: Callback::new(move |(warnings, now): (Option<Vec<String>>, Instant)| {
            update(&tick_owner, visible, deadline, |owner| {
                if let Some(warnings) = warnings {
                    for message in owner.warnings.observe(&warnings) {
                        owner.state.push(ToastLevel::Warning, message.clone(), now);
                    }
                }
                owner.state.expire(now);
            });
        }),
        dispose: Callback::new(move |()| {
            let mut owner = owner.borrow_mut();
            owner.disposed = true;
            owner.state = state::State::default();
            owner.warnings = warnings::Tail::default();
        }),
    }
}
fn now() -> Instant {
    tokio::time::Instant::now().into_std()
}
fn update(
    owner: &RefCell<Mounted>,
    mut visible: Signal<Vec<Toast>>,
    deadline: Option<Signal<Option<Instant>>>,
    apply: impl FnOnce(&mut Mounted),
) {
    let mut owner = owner.borrow_mut();
    if owner.disposed {
        return;
    }
    apply(&mut owner);
    let rows = owner.state.visible();
    let changed = visible.try_peek().is_ok_and(|value| *value != rows);
    if changed && let Ok(mut value) = visible.try_write() {
        *value = rows;
    }
    if let Some(mut deadline) = deadline {
        let next = owner.state.deadline();
        if *deadline.peek() != next {
            deadline.set(next);
        }
    }
}
/// Install app-backed notifications for an isolated fixture without a scheduler.
/// The production editor advances these callbacks from its existing bridge.
pub fn use_provider() -> ToastQueue {
    let visible = use_signal(Vec::new);
    let queue = use_hook(|| mount(visible, None));
    use_context_provider(|| queue);
    use_drop(move || queue.dispose.call(()));
    queue
}
/// Install the standalone gallery's deadline-driven notification preview.
/// This scheduler is never installed by the production editor.
#[must_use]
pub fn use_preview() -> ToastQueue {
    preview::use_preview()
}

#[cfg(test)]
mod tests;
