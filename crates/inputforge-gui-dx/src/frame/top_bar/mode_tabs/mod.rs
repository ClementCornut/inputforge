mod add_inline;
mod context_menu;
mod delete_dialog;
#[cfg(test)]
mod events_tests;
mod rename_inline;
#[cfg(test)]
mod tests;
use crate::{
    components::{TabButton, TabsList, TabsRoot},
    context::AppContext,
    frame::view_state::ViewState,
    mode_management::{Operation, Request, Scope, Transition},
};
use context_menu::{AnchorRect, MenuTarget};
pub(crate) use delete_dialog::{ModeDeleteDialog, ModeDeleteSignal, ModeFocusSignal};
use dioxus::prelude::*;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for bound event closures"
)]
pub(crate) fn ModeTabs() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    #[cfg(test)]
    tests::provide_bindings();
    let bindings = use_context::<crate::mapping_edits::Bindings>().modes;
    let model = ctx.meta.read().mode_management.clone();
    let scope = Scope {
        revision: model.revision,
        profile: ctx.meta.read().profile_id.clone(),
        editing: view.editing_mode.read().clone(),
    };
    let mut adding = use_signal(|| None::<Scope>);
    let mut renaming = use_signal(|| None::<(Scope, String)>);
    let mut menu = use_signal(|| None::<MenuTarget>);
    let mut error = use_signal(|| None::<String>);
    let mut delete_target = use_context::<ModeDeleteSignal>().0;
    let mut focus = use_context::<ModeFocusSignal>().0;
    let select_scope = scope.clone();
    let add_scope = scope.clone();
    rsx! {
        div { class:"if-mode-tabs-outer",
            TabsRoot {
                value:view.editing_mode,
                onchange:move |name:String| {
                    match bindings.dispatch.call(Request {scope:select_scope.clone(),operation:Operation::Select(name)}) {
                        Ok(_) => error.set(None), Err(e) => error.set(Some(e)),
                    }
                },
                focus_request:focus,
                TabsList { class:"if-mode-tabs-wrap".to_owned(),aria_label:"Editing mode".to_owned(),
                    for (index,row) in model.rows.iter().enumerate() {
                        {
                            let name = row.name.clone();
                            let mouse_name = name.clone();
                            let key_name = name.clone();
                            let mouse_scope = scope.clone();
                            let key_scope = scope.clone();
                            let tab_id = format!("mode-tab-{index}");
                            let key_id = tab_id.clone();
                            let open = menu.read().as_ref().is_some_and(|m| m.name == name);
                            if let Some((rename_scope,from)) = renaming.read().as_ref().filter(|(_,n)| n == &name) {
                                rsx! { rename_inline::RenameInline { key:"{name}",from:from.clone(),scope:rename_scope.clone(),state:renaming,pending_focus:focus } }
                            } else {
                                rsx! { TabButton {
                                    key:"{name}",id:name.clone(),label:name.clone(),dom_id:tab_id,title:name.clone(),data_mode:name.clone(),
                                    running:row.running,running_sr_label:"Engine running".to_owned(),aria_haspopup:"menu".to_owned(),aria_expanded:open,
                                    aria_controls:open.then(|| format!("mode-tab-menu-{index}")),
                                    oncontextmenu:move |evt:MouseEvent| {
                                        evt.prevent_default();evt.stop_propagation();
                                        let p = evt.client_coordinates();
                                        menu.set(Some(MenuTarget {scope:mouse_scope.clone(),name:mouse_name.clone(),anchor:AnchorRect {left:p.x,bottom:p.y}}));
                                    },
                                    onkeydown:move |evt:KeyboardEvent| {
                                        if evt.key() == Key::Delete {
                                            evt.prevent_default();evt.stop_propagation();
                                            match bindings.dispatch.call(Request {scope:key_scope.clone(),operation:Operation::BeginDelete(key_name.clone())}) {
                                                Ok(Transition::Delete(target)) => {delete_target.set(Some(target));error.set(None);}
                                                Err(e) => error.set(Some(e)), _ => {},
                                            }
                                        } else if evt.key() == Key::F10 && evt.modifiers().shift() {
                                            evt.prevent_default();evt.stop_propagation();
                                            let id = key_id.clone();let name = key_name.clone();let scope = key_scope.clone();
                                            spawn(async move {
                                                let mut handle = document::eval(&format!("var el = document.getElementById('{id}'); if (!el) {{ dioxus.send([0,0]); return; }} var r = el.getBoundingClientRect(); dioxus.send([r.left,r.bottom]);"));
                                                if let Ok([left,bottom]) = handle.recv::<[f64;2]>().await {menu.set(Some(MenuTarget {scope,name,anchor:AnchorRect {left,bottom}}));}
                                            });
                                        }
                                    },
                                } }
                            }
                        }
                    }
                }
                if let Some(scope) = adding.read().clone() {
                    add_inline::AddInline {scope,open:adding,pending_focus:focus}
                } else {
                    button {r#type:"button",class:"if-mode-tab--add",disabled:!model.can_add,"aria-label":"Add mode",
                        onclick:move |_| {
                            match bindings.dispatch.call(Request {scope:add_scope.clone(),operation:Operation::BeginAdd}) {
                                Ok(Transition::Add) => {adding.set(Some(add_scope.clone()));error.set(None);}
                                Err(e) => error.set(Some(e)), _=>{},
                            }
                        },"+"
                    }
                }
                if let Some(target) = menu.read().clone() {
                    if let Some((index,row)) = model.rows.iter().enumerate().find(|(_,r)| r.name == target.name) {
                        context_menu::ModeTabContextMenu {
                            target,tab_idx:index,row:row.clone(),open:menu,
                            on_transition:move |(scope,transition):(Scope,Transition)| {
                                match transition {
                                    Transition::Rename(name) => renaming.set(Some((scope,name))),
                                    Transition::Delete(target) => delete_target.set(Some(target)),
                                    Transition::Close(Some(name)) => focus.set(Some(name)),
                                    _=>{},
                                }
                            },
                        }
                    }
                }
            }
            if let Some(message) = error.read().as_ref() { span {role:"alert",class:"if-mode-tab__error","{message}"} }
        }
    }
}
