//! Shell-level mode confirmation rendering and focus callback wiring.
use crate::mode_management::ui::{DeleteRequest, Shell};
use dioxus::prelude::*;
fn count_label(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
#[component]
pub(crate) fn ModeDeleteDialog() -> Element {
    let shell = use_context::<Shell>();
    let target = shell.delete;
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let error = use_signal(|| None::<String>);
    let mut open = use_signal(|| false);
    use_effect(move || {
        let want = target.read().is_some();
        if *open.peek() != want {
            open.set(want);
        }
    });
    let current = target.read().clone();
    let name = current
        .as_ref()
        .map_or_else(String::new, |t| t.name.clone());
    let modes = usize::from(current.is_some());
    let mappings = current.as_ref().map_or(0, |t| t.mappings);
    let modes_label = count_label(modes, "mode", "modes");
    let mappings_label = count_label(mappings, "mapping", "mappings");
    let cancel = use_callback(move |()| {
        bindings.delete.call(DeleteRequest {
            shell,
            error,
            confirm: false,
        })
    });
    rsx! {
        crate::components::DialogRoot {open,onclose:move |()| cancel.call(()),
            crate::components::DialogTitle {"Delete mode"}
            crate::components::DialogDescription {"Delete '{name}'?"}
            crate::components::DialogBody {
                crate::components::ReadoutCaption {class:"if-modetab-delete-confirm__counts",span {strong {"{modes}"}" {modes_label}"} span {strong {"{mappings}"}" {mappings_label}"}}
                if let Some(message)=error.read().as_ref(){crate::components::InlineFeedback {tone:crate::components::FeedbackTone::Error,alert:true,"{message}"}}
            }
            crate::components::DialogFooter {
                crate::components::Button {variant:crate::components::ButtonVariant::Ghost,
                    onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                    onclick:move |_| { let _ = cancel.call(()); },"Cancel"
                }
                crate::components::Button {variant:crate::components::ButtonVariant::Danger,
                    onclick:move |_| {
                        bindings.delete.call(DeleteRequest {shell,error,confirm:true});
                    },"Confirm"
                }
            }
        }
    }
}
