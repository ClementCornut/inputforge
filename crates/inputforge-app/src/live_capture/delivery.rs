//! Confirm ambiguous controls and deliver only to the current consumer.
use super::{Controller, confirmation, policy};
use dioxus::prelude::*;

impl Controller {
    /// Revalidate and deliver after releasing the polling guard.
    pub fn deliver(&self) {
        let candidate = {
            let inner = self.inner.borrow();
            let Some(armed) = inner.armed.as_ref() else {
                return;
            };
            if armed.cancelled {
                drop(inner);
                self.finish(false, true);
                return;
            }
            let Some(address) = armed.candidate.clone() else {
                return;
            };
            (
                address,
                armed.request,
                armed.target.clone(),
                armed.generation,
                armed.layouts.clone(),
            )
        };
        let (address, request, target, generation, layouts) = candidate;
        let publication = self.inner.borrow().publication;
        let session = {
            let Some(state) = self.state.try_read() else {
                return;
            };
            if generation != Some(state.session.generation)
                || !state.session.ready
                || layouts != policy::layouts(&state)
                || !policy::present(&state, &address)
                || !request
                    .target
                    .try_peek()
                    .is_ok_and(|current| *current == target)
                || !publication.is_some_and(|p| policy::navigation(p, &target))
                || !policy::valid(&state, &target)
            {
                drop(state);
                self.finish(false, true);
                return;
            }
            state.session.clone()
        };
        if target.mapping.is_some()
            && let Err(error) = confirmation::confirm_binding(&self.commands, &session, &address)
        {
            tracing::warn!(name: "capture.confirm.failed", %error, "live capture confirmation failed");
            self.finish(false, true);
            return;
        }
        self.finish(true, false);
        request.on_capture.call(address);
    }
}
