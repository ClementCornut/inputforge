//! Revalidate pending keyboard completion before invoking the existing GUI handler.
use super::{Controller, policy};
use dioxus::prelude::{ReadableExt, ScopeId, Signal, WritableExt};
use inputforge_gui_dx::keyboard_capture::{Request, Target};
impl Controller {
    pub(super) fn valid(&self, request: Request, target: &Target) -> bool {
        request
            .target
            .try_peek()
            .is_ok_and(|current| *current == *target)
            && self
                .inner
                .borrow()
                .publication
                .is_some_and(|p| policy::navigation(p, target))
            && policy::valid(&self.state.read(), target)
    }
    pub(super) fn deliver(&self, owner: ScopeId) {
        let completion = {
            let inner = self.inner.borrow();
            let Some(armed) = inner.armed.as_ref().filter(|a| a.request.owner == owner) else {
                return;
            };
            (armed.request, armed.target.clone(), armed.pending.clone())
        };
        let (request, target, combo) = completion;
        if !self.valid(request, &target) {
            self.finish(None);
            return;
        }
        let Some(combo) = combo else {
            return;
        };
        self.inner.borrow_mut().armed = None;
        set(request.pending, false);
        // Keep display synchronization suspended until the existing dispatch handler returns.
        request.on_commit.call(combo);
        if self
            .inner
            .borrow()
            .armed
            .as_ref()
            .is_none_or(|a| a.request.owner != owner)
        {
            set(request.active, false);
        }
    }
}
pub(super) fn set<T: 'static>(mut signal: Signal<T>, value: T) {
    if let Ok(mut current) = signal.try_write() {
        *current = value;
    }
}
