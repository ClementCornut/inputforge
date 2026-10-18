mod add_inline;
mod context_menu;
mod delete_dialog;
#[cfg(test)]
mod events_tests;
mod rename_inline;
#[cfg(test)]
mod tests;
use crate::{
    components::{
        ButtonSize, ButtonVariant, FeedbackTone, IconButton, InlineFeedback, TabButton, TabsList,
        TabsRoot,
    },
    context::AppContext,
    frame::view_state::ViewState,
    mode_management::{
        Scope,
        ui::{Event as UiEvent, Publication, Request},
    },
};

pub(crate) use delete_dialog::ModeDeleteDialog;
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
    let shell = use_context::<crate::mode_management::ui::Shell>();
    let publication = Publication {
        adding: use_signal(|| None),
        renaming: use_signal(|| None),
        menu: use_signal(|| None),
        error: use_signal(|| None),
        delete: shell.delete,
        focus: shell.focus,
    };
    let adding = publication.adding;
    let renaming = publication.renaming;
    let menu = publication.menu;
    let error = publication.error;
    let focus = publication.focus;
    let select_scope = scope.clone();
    let add_scope = scope.clone();
    rsx! {
        div { class:"if-mode-tabs-outer",
            TabsRoot {
                value:view.editing_mode,
                onchange:move |name:String| {bindings.ui.call(Request {publication,scope:select_scope.clone(),event:UiEvent::Select(name)});},
                focus_request:focus,
                TabsList { compact: true, scrollable: true, class:"if-mode-tabs-wrap".to_owned(),aria_label:"Editing mode".to_owned(),
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
                            if renaming.read().as_ref().is_some_and(|(_,n)| n == &name) {
                                VNode::empty()
                            } else {
                                rsx! { TabButton {
                                    truncate:true,key:"{name}",id:name.clone(),label:name.clone(),dom_id:tab_id,title:name.clone(),data_mode:name.clone(),
                                    running:row.running,running_sr_label:"Engine running".to_owned(),aria_haspopup:"menu".to_owned(),aria_expanded:open,
                                    aria_controls:open.then(|| format!("mode-tab-menu-{index}")),
                                    oncontextmenu:move |evt| {bindings.ui.call(Request {publication,scope:mouse_scope.clone(),event:UiEvent::Context(mouse_name.clone(),evt)});},
                                    onkeydown:move |evt| {bindings.ui.call(Request {publication,scope:key_scope.clone(),event:UiEvent::Key {name:key_name.clone(),dom_id:key_id.clone(),event:evt}});},
                                } }
                            }
                        }
                    }
                }
                if let Some((rename_scope,from)) = renaming.read().clone() {
                    if let Some(index) = model.rows.iter().position(|row| row.name == from) {
                        rename_inline::RenameInline {key:"{from}",from,editor_id:format!("mode-tab-{index}-name"),scope:rename_scope,publication}
                    }
                }
                if let Some(scope) = adding.read().clone() {
                    add_inline::AddInline {scope,publication}
                } else {
                    IconButton { icon: crate::icons::Icon::Plus, label: "Add mode", variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
                        disabled: !model.can_add,
                        onclick:move |_| {bindings.ui.call(Request {publication,scope:add_scope.clone(),event:UiEvent::BeginAdd});},
                    }
                }
                if let Some(target) = menu.read().clone() {
                    if let Some((index,row)) = model.rows.iter().enumerate().find(|(_,r)| r.name == target.name) {
                        context_menu::ModeTabContextMenu {
                            target,tab_idx:index,row:row.clone(),publication,
                        }
                    }
                }
            }
            if let Some(message) = error.read().as_ref() { InlineFeedback {class:"if-mode-tabs__feedback",tone:FeedbackTone::Error,alert:true,"{message}"} }
        }
    }
}

/// Allocate only the shell's mode presentation destinations.
pub(crate) fn provide_publication(
    delete: Signal<Option<crate::mode_management::Target>>,
    focus: Signal<Option<String>>,
) {
    use_context_provider(|| crate::mode_management::ui::Shell { delete, focus });
}
