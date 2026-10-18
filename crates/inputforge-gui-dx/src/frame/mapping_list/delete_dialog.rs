//! Mapping deletion dialog retains app confirmation policy.
use crate::mapping_edits::Bindings;
use crate::mapping_rail::Target;
use dioxus::prelude::*;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn DeleteDialogMount(delete_target: Signal<Option<Target>>) -> Element {
    let mut dialog_open: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let want = delete_target.read().is_some();
        if *dialog_open.peek() != want {
            dialog_open.set(want);
        }
    });
    let display_name = delete_target
        .read()
        .as_ref()
        .and_then(|t| t.before.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    let bindings = use_context::<Bindings>();
    let feedback = use_signal(|| None::<String>);
    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| bindings.ui.delete.call(crate::mapping_edits::ui::DeleteRequest {
                target: delete_target,
                error: feedback,
                confirm: false,
            }),
            crate::components::DialogTitle {

                "Delete mapping"
            }
            crate::components::DialogBody {

                "Delete '{display_name}'?"
                if let Some(error) = feedback.read().as_ref() {
                    crate::components::InlineFeedback {
                        tone: crate::components::FeedbackTone::Error,
                        alert: true,
                        "{error}"
                    }
                }
            }
            crate::components::DialogFooter {

                crate::components::Button {
                    variant: crate::components::ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick: move |_| {
                        bindings
                            .ui
                            .delete
                            .call(crate::mapping_edits::ui::DeleteRequest {
                                target: delete_target,
                                error: feedback,
                                confirm: false,
                            });
                    },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Danger,
                    onclick: move |_| {
                        bindings
                            .ui
                            .delete
                            .call(crate::mapping_edits::ui::DeleteRequest {
                                target: delete_target,
                                error: feedback,
                                confirm: true,
                            });
                    },
                    "Delete"
                }
            }
        }
    }
}
