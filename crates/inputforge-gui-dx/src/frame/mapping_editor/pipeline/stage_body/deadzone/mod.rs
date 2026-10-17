//! Rendering and supplied callbacks for the deadzone editor.
pub(crate) mod rendering;
pub(crate) mod state;
#[cfg(test)]
mod tests;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;
use super::instruments::{self, wiring::use_instrument};
use crate::{context::AppContext, frame::MappingKey, mapping_edits::StageId};
use dioxus::prelude::*;
use inputforge_core::{action::Action, processing::deadzone::DeadzoneConfig};
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant event qualifications."
)]
pub(crate) fn DeadzoneBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    config: DeadzoneConfig,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let (view, send, on_mounted, plot_dom_id) = use_instrument(
        &mapping_key,
        &stage_id,
        Action::Deadzone {
            config: config.clone(),
        },
    );
    let snapshot = view.read().clone();
    let Action::Deadzone { config } = &snapshot.value else {
        return rsx! {};
    };
    let live_value = instruments::live_axis::value(&stage_id, &mapping_key, &ctx);
    let dragging_attr = snapshot.dragging.is_some().to_string();
    let hovered_attr = snapshot.hovered.is_some().to_string();
    let on_key = move |evt: KeyboardEvent| {
        send.call(crate::instruments::Event::Key {
            key: evt.key().to_string(),
            shift: evt.modifiers().shift(),
        });
        if view.peek().prevent_default {
            evt.prevent_default();
        }
    };
    let body = state::BodyState {
        dragging: snapshot
            .dragging
            .map(|handle| state::DragInProgress { handle }),
        hovered_handle: snapshot.hovered,
        focused_handle: snapshot.focused,
        pending_split: snapshot.split.then_some((1, 2)),
    };
    rsx! {
        div{class:"if-deadzone",
            toolbar::Toolbar{view:snapshot.clone(),send}
            div{class:"if-deadzone__plot-frame",id:"{plot_dom_id}",tabindex:"0","aria-label":"deadzone curve",
                "data-hovered":"{hovered_attr}","data-dragging":"{dragging_attr}",
                onmounted:on_mounted,onkeydown:on_key,onfocusout:move |_|send.call(crate::instruments::Event::Blur),
                {rendering::render_plot(config,&body,live_value)}
            }
            if let Some(error)=snapshot.error { div{class:"if-field__error",role:"alert","{error}"} }
        }
    }
}
