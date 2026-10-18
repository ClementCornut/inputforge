//! Keep native typing browser-owned; only external resets write the live value.
use dioxus::prelude::*;
use inputforge_gui_dx::controls::{TextProps, TextView};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

struct Sync {
    external: String,
    pending: VecDeque<String>,
}
impl Sync {
    fn update(&mut self, value: String) -> Option<String> {
        if let Some(index) = self.pending.iter().position(|raw| raw == &value) {
            self.pending.drain(..=index);
            self.external = value;
            None
        } else if value == self.external {
            None
        } else {
            self.pending.clear();
            self.external.clone_from(&value);
            Some(value)
        }
    }
}
pub(super) fn use_text(props: TextProps) -> TextView {
    let owner = use_hook(|| uuid::Uuid::new_v4().to_string());
    let state = use_hook(|| {
        Rc::new(RefCell::new(Sync {
            external: props.value.peek().clone(),
            pending: VecDeque::new(),
        }))
    });
    let effect_state = Rc::clone(&state);
    let effect_owner = owner.clone();
    // Track replacement props and the live signal, including resets before the next render.
    let external = (props.value)();
    use_effect(use_reactive((&external,), move |_| {
        let value = (props.value)();
        if let Some(value) = effect_state.borrow_mut().update(value) {
            let value = serde_json::to_string(&value).expect("text serializes as JSON");
            let _ = document::eval(&format!(
                "const el = document.querySelector('[data-if-text-owner=\"{effect_owner}\"]'); \
                 if (el instanceof HTMLInputElement && el.value !== {value}) el.value = {value};"
            ));
        }
    }));
    let input = use_callback(move |event: FormEvent| {
        if !props.disabled
            && let Some(handler) = props.oninput
        {
            let raw = event.value();
            state.borrow_mut().pending.push_back(raw.clone());
            handler.call(event);
            let observed = props.value.peek().clone();
            if observed == raw {
                let _ = state.borrow_mut().update(observed);
            }
        }
    });
    TextView { owner, input }
}
