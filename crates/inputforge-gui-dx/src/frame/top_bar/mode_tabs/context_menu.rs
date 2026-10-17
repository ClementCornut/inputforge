//! Mode menu rendering, anchor mechanics and app callback wiring.
use crate::{
    components::{AnchoredMenu, MenuAnchor, MenuItem},
    mode_management::{Operation, Request, Row, Scope, Transition},
    overlays::CloseReason,
};
use dioxus::prelude::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AnchorRect {
    pub left: f64,
    pub bottom: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MenuTarget {
    pub scope: Scope,
    pub name: String,
    pub anchor: AnchorRect,
}
#[component]
pub(crate) fn ModeTabContextMenu(
    target: MenuTarget,
    tab_idx: usize,
    row: Row,
    open: Signal<Option<MenuTarget>>,
    on_transition: EventHandler<(Scope, Transition)>,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let mut error = use_signal(|| None::<String>);
    let mut activation = use_signal(|| None::<Transition>);
    let activate_target = target.clone();
    let action = use_callback(move |operation: Operation| {
        let result = bindings.dispatch.call(Request {
            scope: activate_target.scope.clone(),
            operation,
        });
        match result {
            Ok(t) => {
                activation.set(Some(t));
                error.set(None);
            }
            Err(e) => {
                activation.set(None);
                error.set(Some(e));
            }
        }
    });
    let name = target.name.clone();
    let rename = name.clone();
    let delete = name.clone();
    let default = name.clone();
    rsx! {
        AnchoredMenu {
            open:Some(MenuAnchor {x:target.anchor.left,y:target.anchor.bottom}),
            aria_labelledby:format!("mode-tab-{tab_idx}"),class:"if-modetab-context-menu".to_owned(),
            on_close:move |reason:CloseReason| {
                let transition = if matches!(reason,CloseReason::ItemActivated) {
                    let Some(t) = activation.peek().clone() else {return};t
                } else {
                    match bindings.dispatch.call(Request {scope:target.scope.clone(),operation:Operation::CloseMenu {name:target.name.clone(),tab:matches!(reason,CloseReason::Tab)}}) {
                        Ok(t)=>t,Err(e)=>{error.set(Some(e));return;},
                    }
                };
                open.set(None);on_transition.call((target.scope.clone(),transition));
            },
            MenuItem {disabled:!row.can_activate,onclick:move |_| action.call(Operation::Activate(name.clone())),"Activate"}
            MenuItem {disabled:!row.can_rename,onclick:move |_| action.call(Operation::BeginRename(rename.clone())),"Rename"}
            MenuItem {disabled:!row.can_delete,onclick:move |_| action.call(Operation::BeginDelete(delete.clone())),"Delete"}
            MenuItem {disabled:!row.can_default,onclick:move |_| action.call(Operation::Default(default.clone())),"Set as default"}
            if let Some(message) = error.read().as_ref() {span {role:"alert",class:"if-mode-tab__error","{message}"}}
        }
    }
}
