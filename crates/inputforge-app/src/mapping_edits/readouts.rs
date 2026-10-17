//! Expansion events interpreted against the captured prepared readout identity.
use inputforge_gui_dx::readouts::{ExpandState, Expansion};
pub(super) fn expand(request: Expansion) -> ExpandState {
    let mut state = request.state;
    if let Some(index) = request.toggle {
        if state.key.as_ref() == Some(&request.key) && index < request.outputs {
            state.per_output.resize(request.outputs, false);
            state.per_output[index] = !state.per_output[index];
        }
    } else if state.key.as_ref() != Some(&request.key) || state.per_output.len() != request.outputs
    {
        state.key = Some(request.key);
        state.per_output = vec![false; request.outputs];
    }
    state
}
