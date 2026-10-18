//! Inline rename rendering and event wiring.
use crate::{
    components::{FeedbackTone, InlineFeedback, InlineTextEditor},
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
pub(crate) fn RenameInline(
    from: String,
    editor_id: String,
    scope: Scope,
    publication: Publication,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let value = use_signal(|| from.clone());
    let error = use_signal(|| None::<String>);
    let draft = Draft {
        value,
        error,
        from: Some(from.clone()),
    };
    let key_draft = draft.clone();
    let key_scope = scope.clone();
    let input_draft = draft.clone();
    let input_scope = scope.clone();
    let error_id = format!("{editor_id}-error");
    rsx! {
        div {class:"if-mode-tab--rename-inline",
            onkeydown:move |evt| {bindings.ui.call(Request {publication,scope:key_scope.clone(),event:UiEvent::DraftKey {draft:key_draft.clone(),event:evt}});},
            onfocusout:move |_| {bindings.ui.call(Request {publication,scope:scope.clone(),event:UiEvent::Commit {draft:draft.clone(),blur:true}});},
            InlineTextEditor {aria_label:"Mode name",id:editor_id,value:ReadSignal::from(value),invalid:error.read().is_some(),aria_describedby:error.read().as_ref().map(|_| error_id.clone()),aria_errormessage:error.read().as_ref().map(|_| error_id.clone()),
                onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                oninput:move |evt:FormEvent| {bindings.ui.call(Request {publication,scope:input_scope.clone(),event:UiEvent::Input {draft:input_draft.clone(),value:evt.value()}});},
            }
            if let Some(message)=error.read().as_ref(){InlineFeedback {id:"{error_id}",tone:FeedbackTone::Error,alert:true,"{message}"}}
        }
    }
}
