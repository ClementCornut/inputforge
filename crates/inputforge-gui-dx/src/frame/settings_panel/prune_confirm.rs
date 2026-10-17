//! Prune confirmation renders captured app state and forwards resolution.
use crate::{
    components::{
        Button, ButtonVariant, DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
    },
    settings::Intent,
};
use dioxus::prelude::*;
#[component]
pub(crate) fn PruneConfirmDialog() -> Element {
    let c = super::context();
    let view = c.view.read().clone();
    let mut open = use_signal(|| false);
    use_effect(move || {
        let wanted = c.view.read().prune.is_some();
        if *open.peek() != wanted {
            open.set(wanted);
        }
    });
    let Some(pending) = view.prune else {
        return rsx! {};
    };
    rsx! {
        DialogRoot {open, close_on_backdrop_click: false, onclose:move|()|c.send(Intent::Cancel),
            DialogTitle {"Reduce snapshot buffer to {pending.candidate_max}?"}
            DialogDescription {"{pending.will_remove} unpinned snapshots will be deleted from {pending.profile_name}. Pinned snapshots are kept."}
            DialogBody {if let Some(error)=view.error {div {role:"alert","{error}"}}}
            DialogFooter {
                Button {variant:ButtonVariant::Secondary,onclick:move|_|{let _=c.send(Intent::Cancel);},"Cancel"}
                Button {variant:ButtonVariant::Danger,onclick:move|_|{let _=c.send(Intent::Confirm);},"Reduce"}
            }
        }
    }
}
