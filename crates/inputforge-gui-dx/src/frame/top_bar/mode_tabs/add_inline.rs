//! Inline add-mode rendering and event wiring.
use crate::{
    components::{InputSize, TextInput},
    mode_management::{Operation, Request, Scope, Transition},
};
use dioxus::prelude::*;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for bound event closures"
)]
pub(crate) fn AddInline(
    scope: Scope,
    open: Signal<Option<Scope>>,
    pending_focus: Signal<Option<String>>,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let mut value = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let commit_scope = scope.clone();
    let commit = use_callback(move |blur: bool| {
        let raw = value.peek().clone();
        match bindings.dispatch.call(Request {
            scope: commit_scope.clone(),
            operation: Operation::Add { raw, blur },
        }) {
            Ok(Transition::Close(focus)) => {
                if let Some(n) = focus {
                    pending_focus.set(Some(n));
                }
                value.set(String::new());
                error.set(None);
                open.set(None);
            }
            Err(e) => error.set(Some(e)),
            _ => {}
        }
    });
    rsx! {
        div {class:"if-mode-tab if-mode-tab--add-inline",
            onkeydown:move |evt:KeyboardEvent| match evt.key() {
                Key::Enter=>{evt.prevent_default();commit.call(false);}
                Key::Escape=>{evt.prevent_default();if bindings.dispatch.call(Request {scope:scope.clone(),operation:Operation::Cancel(None)}).is_ok(){open.set(None);}}
                _=>{},
            },
            onfocusout:move |_| commit.call(true),
            TextInput {value:ReadSignal::from(value),size:InputSize::Sm,placeholder:"New mode name".to_owned(),invalid:error.read().is_some(),aria_describedby:error.read().as_ref().map(|_| "mode-name-error-add".to_owned()),
                onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                oninput:move |evt:FormEvent| {value.set(evt.value());error.set(None);},
            }
            if let Some(message)=error.read().as_ref(){span {id:"mode-name-error-add",role:"alert","aria-live":"assertive",class:"if-mode-tab__error","{message}"}}
        }
    }
}
