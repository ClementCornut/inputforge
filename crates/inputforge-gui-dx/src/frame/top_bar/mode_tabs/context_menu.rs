//! Mode menu rendering, anchor mechanics and app callback wiring.
use crate::{
    components::{AnchoredMenu, FeedbackTone, InlineFeedback, MenuAnchor, MenuItem},
    mode_management::{
        Operation, Row, Transition,
        ui::{Event as UiEvent, MenuTarget, Publication, Request},
    },
    overlays::CloseReason,
};
use dioxus::prelude::*;
#[component]
pub(crate) fn ModeTabContextMenu(
    target: MenuTarget,
    tab_idx: usize,
    row: Row,
    publication: Publication,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let error = use_signal(|| None::<String>);
    let activation = use_signal(|| None::<Transition>);
    let activate_target = target.clone();
    let action = use_callback(move |operation: Operation| {
        bindings.ui.call(Request {
            publication,
            scope: activate_target.scope.clone(),
            event: UiEvent::MenuAction {
                operation,
                activation,
                error,
            },
        });
    });
    let name = target.name.clone();
    let rename = name.clone();
    let delete = name.clone();
    let default = name.clone();
    rsx! {
        AnchoredMenu {
            id:format!("mode-tab-menu-{tab_idx}"),
            open:Some(MenuAnchor {x:target.left,y:target.bottom}),
            return_focus:format!("mode-tab-{tab_idx}"),aria_labelledby:format!("mode-tab-{tab_idx}"),class:"if-modetab-context-menu".to_owned(),
            on_close:move |reason:CloseReason| {
                bindings.ui.call(Request {publication,scope:target.scope.clone(),event:UiEvent::MenuClose {target:target.clone(),reason,activation,error}});
                publication.menu.peek().is_none()
            },
            MenuItem {disabled:!row.can_activate,onclick:move |_| action.call(Operation::Activate(name.clone())),"Activate"}
            MenuItem {disabled:!row.can_rename,onclick:move |_| action.call(Operation::BeginRename(rename.clone())),"Rename"}
            MenuItem {danger:true,disabled:!row.can_delete,onclick:move |_| action.call(Operation::BeginDelete(delete.clone())),"Delete"}
            MenuItem {disabled:!row.can_default,onclick:move |_| action.call(Operation::Default(default.clone())),"Set as default"}
            if let Some(message) = error.read().as_ref() {InlineFeedback {tone:FeedbackTone::Error,alert:true,"{message}"}}
        }
    }
}
