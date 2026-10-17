//! Bounded notification state with injected monotonic time.
use dioxus::prelude::Key;
use inputforge_gui_dx::toast::{Interaction, Toast, ToastLevel};
use std::time::{Duration, Instant};

// Preserve the existing effective lifetime and visible stack capacity.
const LIFETIME: Duration = Duration::from_secs(8);
const CAPACITY: usize = 5;

#[derive(Debug)]
struct Entry {
    toast: Toast,
    deadline: Instant,
    paused: Option<Instant>,
    hover: bool,
    focus: bool,
}
#[derive(Debug, Default)]
pub(super) struct State {
    entries: Vec<Entry>,
    next_id: u64,
}
impl State {
    pub(super) fn push(&mut self, level: ToastLevel, message: String, now: Instant) {
        self.expire(now);
        if let Some(last) = self.entries.last_mut()
            && last.toast.level == level
            && last.toast.message == message
        {
            last.toast.count = last.toast.count.saturating_add(1);
            last.deadline = now + LIFETIME;
            last.paused = (last.hover || last.focus).then_some(now);
            return;
        }
        if self.entries.len() == CAPACITY {
            self.entries.remove(0);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.entries.push(Entry {
            toast: Toast {
                id,
                level,
                message,
                count: 1,
            },
            deadline: now + LIFETIME,
            paused: None,
            hover: false,
            focus: false,
        });
    }
    pub(super) fn event(&mut self, id: u64, event: &Interaction, now: Instant) {
        self.expire(now);
        let Some(index) = self.entries.iter().position(|entry| entry.toast.id == id) else {
            return;
        };
        let entry = &mut self.entries[index];
        match event {
            Interaction::Dismiss | Interaction::Key(Key::Escape) => {
                self.entries.remove(index);
            }
            Interaction::Key(_) => {}
            Interaction::Hover(active) | Interaction::Focus(active) => {
                match event {
                    Interaction::Hover(_) => entry.hover = *active,
                    _ => entry.focus = *active,
                }
                if entry.hover || entry.focus {
                    entry.paused.get_or_insert(now);
                } else if let Some(start) = entry.paused.take() {
                    entry.deadline += now.saturating_duration_since(start);
                }
            }
        }
    }
    pub(super) fn expire(&mut self, now: Instant) {
        self.entries
            .retain(|entry| entry.paused.is_some() || entry.deadline > now);
    }
    pub(super) fn visible(&self) -> Vec<Toast> {
        self.entries
            .iter()
            .map(|entry| entry.toast.clone())
            .collect()
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.entries
            .iter()
            .filter(|entry| entry.paused.is_none())
            .map(|entry| entry.deadline)
            .min()
    }
}
