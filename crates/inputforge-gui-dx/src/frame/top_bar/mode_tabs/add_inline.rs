//! Inline add-mode rendering and event wiring.
use crate::{
    components::{InputSize, TextInput},
    mode_management::{
        Scope,
        ui::{Draft, Event as UiEvent, Publication, Request},
    },
};
use dioxus::prelude::*;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for bound event closures"
)]
pub(crate) fn AddInline(scope: Scope, publication: Publication) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let value = use_signal(String::new);
    let error = use_signal(|| None::<String>);
    let draft = Draft {
        value,
        error,
        from: None,
    };
    let key_draft = draft.clone();
    let key_scope = scope.clone();
    let input_draft = draft.clone();
    let input_scope = scope.clone();
    rsx! {
        div {class:"if-mode-tab if-mode-tab--add-inline",
            onkeydown:move |evt| {bindings.ui.call(Request {publication,scope:key_scope.clone(),event:UiEvent::DraftKey {draft:key_draft.clone(),event:evt}});},
            onfocusout:move |_| {bindings.ui.call(Request {publication,scope:scope.clone(),event:UiEvent::Commit {draft:draft.clone(),blur:true}});},
            TextInput {value:ReadSignal::from(value),size:InputSize::Sm,placeholder:"New mode name".to_owned(),invalid:error.read().is_some(),aria_describedby:error.read().as_ref().map(|_| "mode-name-error-add".to_owned()),
                onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                oninput:move |evt:FormEvent| {bindings.ui.call(Request {publication,scope:input_scope.clone(),event:UiEvent::Input {draft:input_draft.clone(),value:evt.value()}});},
            }
            if let Some(message)=error.read().as_ref(){span {id:"mode-name-error-add",role:"alert","aria-live":"assertive",class:"if-mode-tab__error","{message}"}}
        }
    }
}
