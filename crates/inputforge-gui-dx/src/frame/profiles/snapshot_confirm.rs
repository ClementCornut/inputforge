//! Destructive snapshot confirmation renders app-owned state.
use crate::{
    components::{Button, ButtonVariant, DialogBody, DialogFooter, DialogRoot, DialogTitle},
    profiles::{Context, Intent, snapshots},
};
use dioxus::prelude::*;
#[component]
pub(super) fn SnapshotConfirm() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let mut open = use_signal(|| true);
    use_effect(move || {
        let wanted = c.view.read().snapshots.restore.is_some();
        if *open.peek() != wanted {
            open.set(wanted);
        }
    });
    rsx! {
        DialogRoot {open,onclose:move|()|c.send(Intent::Snapshot(snapshots::Intent::Cancel)),
            DialogTitle {"Restore snapshot?"}
            DialogBody {
                "This replaces the current profile contents. A recovery snapshot is created before restoring."
                if let Some(error)=view.error {div{role:"alert","{error}"}}
            }
            DialogFooter {
                Button {variant:ButtonVariant::Ghost,onmounted:move|e|c.bindings.focus.call((c.owner,c.workflow,e)),onclick:move|_|{let _=c.send(Intent::Snapshot(snapshots::Intent::Cancel));},"Cancel"}
                Button {variant:ButtonVariant::Danger,onclick:move|_|{let _=c.send(Intent::Snapshot(snapshots::Intent::Confirm));},"Restore"}
            }
        }
    }
}
