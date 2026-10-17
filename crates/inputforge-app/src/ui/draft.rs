//! Numeric edit buffers retain text until acknowledgement or cancellation.
use dioxus::prelude::*;
#[derive(Clone, Debug)]
pub(super) struct Draft {
    pub raw: String,
    dirty: bool,
    attempted: bool,
    suppress_blur: bool,
}
impl Draft {
    pub(super) fn new(external: String) -> Self {
        Self {
            raw: external,
            dirty: false,
            attempted: false,
            suppress_blur: false,
        }
    }
    pub(super) fn sync(&mut self, external: &str, acknowledged: bool) {
        if !self.dirty || acknowledged {
            external.clone_into(&mut self.raw);
            self.dirty = false;
        }
    }
    pub(super) fn edit(&mut self, raw: String) {
        self.raw = raw;
        self.dirty = true;
        self.attempted = false;
        self.suppress_blur = false;
    }
    pub(super) fn take(&mut self) -> Option<String> {
        if !self.dirty || self.attempted {
            return None;
        }
        self.attempted = true;
        Some(self.raw.clone())
    }
    // A commit matching the published value cannot produce a later acknowledgement frame.
    pub(super) fn accept(&mut self) {
        self.dirty = false;
    }
    pub(super) fn enter(&mut self) {
        self.suppress_blur = true;
    }
    pub(super) fn blur(&mut self) -> bool {
        std::mem::take(&mut self.suppress_blur)
    }
    pub(super) fn retry(&mut self) {
        self.attempted = false;
    }
    pub(super) fn cancel(&mut self, external: String) {
        *self = Self::new(external);
    }
    pub(super) fn acknowledged(&self) -> bool {
        self.dirty && self.attempted
    }
}
pub(super) fn blur() {
    let _ = document::eval(
        "const el = document.activeElement; if (el instanceof HTMLInputElement) el.blur();",
    );
}
