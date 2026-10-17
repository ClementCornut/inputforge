//! Rendering and supplied callbacks for the curve editor.
pub(crate) mod rendering;
pub(crate) mod state;
#[cfg(test)]
mod tests;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;
use super::instruments::{self, wiring::use_instrument};
use crate::{context::AppContext, frame::MappingKey, mapping_edits::StageId};
use dioxus::prelude::*;
use inputforge_core::{action::Action, processing::curves::ResponseCurve};
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant event qualifications."
)]
pub(crate) fn ResponseCurveBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    curve: ResponseCurve,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let (view, send, on_mounted, plot_dom_id) = use_instrument(
        &mapping_key,
        &stage_id,
        Action::ResponseCurve {
            curve: curve.clone(),
        },
    );
    let snapshot = view.read().clone();
    let Action::ResponseCurve { curve } = &snapshot.value else {
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
    let body = use_memo(move || {
        let view = view.read();
        let Action::ResponseCurve { curve } = &view.value else {
            return state::BodyState::default();
        };
        state::BodyState {
            cached_path: inputforge_core::processing::curves::sample_curve_path(curve, 200),
            cached_anchors: state::extract_anchors(curve),
            dragging: view
                .dragging
                .map(|point_index| state::DragInProgress { point_index }),
            hovered_point: view.hovered,
            focused_point: view.focused,
        }
    });
    let summary = crate::frame::mapping_editor::pipeline::stage::stage_summary_for(
        &snapshot.value,
        &ctx.config.read(),
    );
    rsx! {
        div{class:"if-curve",
            "data-summary":"{summary}",
            toolbar::Toolbar{view:snapshot.clone(),send}
            div{class:"if-curve__plot-frame",id:"{plot_dom_id}",tabindex:"0","aria-label":"response curve",
                "data-hovered":"{hovered_attr}","data-dragging":"{dragging_attr}",
                onmounted:on_mounted,onkeydown:on_key,onfocusout:move |_|send.call(crate::instruments::Event::Blur),
                {rendering::render_plot(curve,&body.read(),live_value,240.0)}
            }
            if let Some(error)=snapshot.error { div{class:"if-field__error",role:"alert","{error}"} }
        }
    }
}
