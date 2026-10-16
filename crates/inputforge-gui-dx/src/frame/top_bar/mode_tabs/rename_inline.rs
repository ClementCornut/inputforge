//! Inline rename rendering and event wiring.
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
pub(crate) fn RenameInline(
    from: String,
    scope: Scope,
    state: Signal<Option<(Scope, String)>>,
    pending_focus: Signal<Option<String>>,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let mut value = use_signal(|| from.clone());
    let mut error = use_signal(|| None::<String>);
    let commit_scope = scope.clone();
    let commit_from = from.clone();
    let commit = use_callback(move |blur: bool| {
        match bindings.dispatch.call(Request {
            scope: commit_scope.clone(),
            operation: Operation::Rename {
                from: commit_from.clone(),
                raw: value.peek().clone(),
                blur,
            },
        }) {
            Ok(Transition::Close(focus)) => {
                if let Some(n) = focus {
                    pending_focus.set(Some(n));
                }
                error.set(None);
                state.set(None);
            }
            Err(e) => error.set(Some(e)),
            _ => {}
        }
    });
    let error_id = format!("mode-name-error-{from}");
    rsx! {
        div {class:"if-mode-tab if-mode-tab--rename-inline",
            onkeydown:move |evt:KeyboardEvent| match evt.key() {
                Key::Enter=>{evt.prevent_default();commit.call(false);}
                Key::Escape=>{evt.prevent_default();if let Ok(Transition::Close(focus))=bindings.dispatch.call(Request {scope:scope.clone(),operation:Operation::Cancel(Some(from.clone()))}){if let Some(n)=focus {pending_focus.set(Some(n));} state.set(None);}}
                _=>{},
            },
            onfocusout:move |_| commit.call(true),
            TextInput {value:ReadSignal::from(value),size:InputSize::Sm,invalid:error.read().is_some(),aria_describedby:error.read().as_ref().map(|_| error_id.clone()),
                onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                oninput:move |evt:FormEvent| {value.set(evt.value());error.set(None);},
            }
            if let Some(message)=error.read().as_ref(){span {id:"{error_id}",role:"alert","aria-live":"assertive",class:"if-mode-tab__error","{message}"}}
        }
    }
}
