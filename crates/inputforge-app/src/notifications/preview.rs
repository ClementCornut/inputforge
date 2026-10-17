//! Standalone gallery deadlines reuse production state without an engine poller.
use dioxus::{core::Task, prelude::*};
use inputforge_gui_dx::toast::ToastQueue;
use std::{cell::RefCell, rc::Rc};

pub(super) fn use_preview() -> ToastQueue {
    let visible = use_signal(Vec::new);
    let deadline = use_signal(|| None);
    let queue = use_hook(|| super::mount(visible, Some(deadline)));
    use_context_provider(|| queue);
    let timer = use_hook(|| Rc::new(RefCell::new(None::<Task>)));
    let cleanup = Rc::clone(&timer);
    use_drop(move || {
        queue.dispose.call(());
        if let Some(task) = cleanup.borrow_mut().take() {
            task.cancel();
        }
    });
    use_effect(move || {
        if let Some(task) = timer.borrow_mut().take() {
            task.cancel();
        }
        if let Some(next) = *deadline.read() {
            *timer.borrow_mut() = Some(spawn(async move {
                tokio::time::sleep_until(next.into()).await;
                queue.tick.call((None, super::now()));
            }));
        }
    });
    queue
}
