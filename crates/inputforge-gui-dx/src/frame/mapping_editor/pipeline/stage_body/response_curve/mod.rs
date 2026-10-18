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
    let glow_id = format!("{plot_dom_id}-glow");
    let onfocusout = move |_| send.call(crate::instruments::Event::Blur);
    rsx! {
        div {
            class: "if-curve",
            "data-summary": summary.clone(),
            toolbar::Toolbar {
                view: snapshot.clone(),
                send,
            }
            crate::components::PlotFrame {
                class: "if-curve__plot-frame",
                id: plot_dom_id,
                label: "response curve",
                description: "Edit the response curve control points.",
                instructions: "Use Tab to cycle handles and arrow keys to adjust. Hold Shift for larger adjustments.",
                value_text: summary.clone(),
                hovered: snapshot.hovered.is_some(),
                dragging: snapshot.dragging.is_some(),
                onmounted: on_mounted,
                onkeydown: on_key,
                onfocusout,
                {rendering::render_plot(curve, &body.read(), live_value, 240.0, &glow_id)}
            }
            if let Some(error) = snapshot.error {
                crate::components::InlineFeedback {
                    tone: crate::components::FeedbackTone::Error,
                    alert: true,
                    "{error}"
                }
            }
        }
    }
}
