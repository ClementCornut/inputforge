//! Add-pad capture coordination uses the existing app-owned state machine.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mapping_edits::ui,
    mapping_rail::{AddIntent, AddRequest, Phase},
};
impl Controller {
    pub(super) fn add_submit(
        &self,
        mut state: Signal<inputforge_gui_dx::mapping_rail::AddView>,
        mut name: Signal<String>,
        intent: AddIntent,
    ) -> bool {
        if state.try_peek().is_err() {
            return false;
        }
        let next = self.add(AddRequest {
            view: state.peek().clone(),
            intent,
        });
        let capturing = next.phase == Phase::Capturing;
        if next.phase == Phase::Resting {
            name.set(String::new());
        }
        if *state.peek() != next {
            state.set(next);
        }
        capturing
    }
    pub(super) fn add_ui(&self, mut p: ui::Add, event: ui::AddEvent) {
        if p.state.try_peek().is_err() {
            return;
        }
        match event {
            ui::AddEvent::Intent(intent) => {
                let refresh = matches!(intent, AddIntent::Refresh);
                if self.add_submit(p.state, p.name, intent) && refresh {
                    p.capture.start.call(());
                }
            }
            ui::AddEvent::Begin { scope, source } => {
                if let Some(source) = &source {
                    p.name.set(format!(
                        "{} (copy)",
                        source.before.name.as_deref().unwrap_or("(unnamed)")
                    ));
                }
                if self.add_submit(p.state, p.name, AddIntent::Start { scope, source }) {
                    p.capture.start.call(());
                }
            }
            ui::AddEvent::Observe {
                scope,
                mut pending,
                mut expanded,
            } => {
                let source = pending.peek().clone();
                if source.is_some() {
                    pending.set(None);
                    self.add_ui(
                        p,
                        ui::AddEvent::Begin {
                            scope: scope.clone(),
                            source,
                        },
                    );
                }
                let force = *expanded.peek();
                if force {
                    expanded.set(false);
                    self.add_ui(
                        p,
                        ui::AddEvent::Begin {
                            scope,
                            source: None,
                        },
                    );
                }
            }
            ui::AddEvent::Reconcile => {
                self.add_submit(p.state, p.name, AddIntent::Reconcile);
                if p.state.peek().phase == Phase::Resting && *p.capture.listening.peek() {
                    p.capture.cancel.call(());
                }
            }
            ui::AddEvent::Key(event) => {
                if event.key() == Key::Enter && matches!(p.state.peek().phase, Phase::Captured(_)) {
                    event.prevent_default();
                    self.add_ui(p, ui::AddEvent::Commit);
                }
            }
            ui::AddEvent::Input(value) => p.name.set(value),
            ui::AddEvent::Commit => {
                let raw = p.name.peek().clone();
                self.add_submit(p.state, p.name, AddIntent::Commit(raw));
            }
            ui::AddEvent::Cancel => {
                p.capture.cancel.call(());
                self.add_submit(p.state, p.name, AddIntent::Cancel);
            }
        }
    }
}
