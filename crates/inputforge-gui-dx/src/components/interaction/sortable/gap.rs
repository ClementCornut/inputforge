//! Gap markup and app-prepared event wiring.
use super::state::SortableState;
use dioxus::prelude::*;
/// Render one insertion slot using shared app policy.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus emits redundant event qualifications"
)]
pub fn SortableGap<G: 'static + Clone + PartialEq>(
    state: SortableState<G>,
    gap_index: usize,
    group: G,
    on_drop: EventHandler<usize>,
) -> Element {
    let view = (state.bindings.gap)(crate::sortable::GapProps {
        state,
        gap_index,
        group,
        on_drop,
    });
    rsx! { li { class: view.class, role:"presentation", "aria-hidden":"true",
        ondragover:view.over, ondragleave:view.leave, ondragend:view.end, ondrop:view.drop,
    } }
}
