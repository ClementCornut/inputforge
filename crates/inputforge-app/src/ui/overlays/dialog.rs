//! Controlled dismissal waits for the workflow's result before closing.
use dioxus::prelude::*;
use inputforge_gui_dx::overlays::{DialogProps, DialogView};
pub(super) fn use_dialog(props: DialogProps) -> DialogView {
    let mut open = props.open;
    let id = props.id;
    let visibility_id = id.clone();
    use_effect(move || {
        let action = if open() {
            "if (!d.open) d.showModal();"
        } else {
            "if (d.open) d.close();"
        };
        let _ = document::eval(&format!(
            "{{ const d = document.getElementById({visibility_id:?}); if (d) {{ {action} }} }}"
        ));
    });
    let key = use_hook(|| format!("if-dialog-cancel-{}", uuid::Uuid::new_v4().simple()));
    let mount_key = key.clone();
    use_effect(move || {
        let script = include_str!("../../../assets/dialog_cancel.js")
            .replace("__KEY__", &format!("{mount_key:?}"))
            .replace("__ID__", &format!("{id:?}"));
        let _ = document::eval(&script);
    });
    use_drop(move || {
        let _ = document::eval(&format!("window[{key:?}]?.(); delete window[{key:?}];"));
    });
    let dismiss = use_callback(move |()| {
        if open() && props.dismissible && props.onclose.call(()) {
            open.set(false);
        }
    });
    DialogView {
        cancel: use_callback(move |event: Event<CancelData>| {
            event.prevent_default();
            event.stop_propagation();
            dismiss.call(());
        }),
        backdrop: use_callback(move |_| {
            if props.backdrop {
                dismiss.call(());
            }
        }),
        panel: use_callback(move |event: MouseEvent| event.stop_propagation()),
    }
}
