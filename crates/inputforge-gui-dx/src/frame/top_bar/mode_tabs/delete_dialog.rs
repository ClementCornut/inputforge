//! Shell-level mode confirmation rendering and focus callback wiring.
use crate::mode_management::{Operation, Request, Target, Transition};
use dioxus::prelude::*;
#[derive(Clone, Copy)]
pub(crate) struct ModeDeleteSignal(pub Signal<Option<Target>>);
#[derive(Clone, Copy)]
pub(crate) struct ModeFocusSignal(pub Signal<Option<String>>);
fn count_label(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
#[component]
pub(crate) fn ModeDeleteDialog() -> Element {
    let mut target = use_context::<ModeDeleteSignal>().0;
    let mut focus = use_context::<ModeFocusSignal>().0;
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let mut error = use_signal(|| None::<String>);
    let mut open = use_signal(|| false);
    use_effect(move || {
        let want = target.read().is_some();
        if *open.peek() != want {
            open.set(want);
            error.set(None);
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
        let Some(t) = target.peek().clone() else {
            return;
        };
        match bindings.dispatch.call(Request {
            scope: t.scope,
            operation: Operation::Cancel(Some(t.name)),
        }) {
            Ok(Transition::Close(f)) => {
                if let Some(n) = f {
                    focus.set(Some(n));
                }
                target.set(None);
                error.set(None);
            }
            Err(e) => error.set(Some(e)),
            _ => {}
        }
    });
    rsx! {
        crate::components::DialogRoot {open,onclose:move |()| cancel.call(()),
            crate::components::DialogTitle {"Delete mode"}
            crate::components::DialogBody {"Delete '{name}'?"
                div {class:"if-modetab-delete-confirm__counts",span {strong {"{modes}"}" {modes_label}"} span {strong {"{mappings}"}" {mappings_label}"}}
                if let Some(message)=error.read().as_ref(){span {role:"alert",class:"if-mode-tab__error","{message}"}}
            }
            crate::components::DialogFooter {
                crate::components::Button {variant:crate::components::ButtonVariant::Ghost,
                    onmounted:move |evt:MountedEvent| {spawn(async move {let _=evt.data().set_focus(true).await;});},
                    onclick:move |_| cancel.call(()),"Cancel"
                }
                crate::components::Button {variant:crate::components::ButtonVariant::Secondary,
                    onclick:move |_| {
                        let Some(t)=target.peek().clone() else{return};
                        match bindings.dispatch.call(Request {scope:t.scope.clone(),operation:Operation::Delete(t)}) {
                            Ok(Transition::Close(f))=>{if let Some(n)=f{focus.set(Some(n));}target.set(None);error.set(None);}
                            Err(e)=>error.set(Some(e)),_=>{},
                        }
                    },"Confirm"
                }
            }
        }
    }
}
