// Rust guideline compliant 2026-05-03

//! Shared `#[cfg(test)]` mount harness for stage-body SSR tests. Builds a
//! minimal `AppContext` + `EditorState` provider stack and renders the
//! caller's RSX to a string. Used by F10 (`response_curve/tests.rs`) and F11
//! (`deadzone/tests.rs`) to keep the per-instrument tests focused on output
//! assertions instead of context construction.
//!
//! Caller contract: pass a `fn() -> Element` (a free fn or a non-capturing
//! closure coerced to a function pointer). Dioxus 0.7's `VirtualDom::new`
//! only accepts function pointers, so a capturing closure cannot be used
//! directly here. To pass per-test inputs, build them inside the function.

use dioxus::prelude::*;
use dioxus_ssr::render;

use crate::context::AppContext;
use crate::frame::mapping_editor::use_editor_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;

#[derive(Clone, Props, PartialEq)]
#[allow(
    unpredictable_function_pointer_comparisons,
    reason = "PartialEq drives Dioxus memoization. SSR mount renders once \
              and discards the dom; the comparison result is irrelevant."
)]
pub(crate) struct MountStageBodyProps {
    /// Function pointer that produces the inner RSX. Passed as a fn ptr
    /// rather than a closure because Dioxus props must be `Clone +
    /// PartialEq + 'static`; `fn` items satisfy all three trivially while
    /// closures generally do not.
    pub body_fn: fn() -> Element,
}

/// Wrapper component that installs the canonical `AppContext` +
/// `EditorState` + live-capture provider stack, then invokes
/// `body_fn` to render the caller's RSX inside that stack.
#[component]
pub(crate) fn MountStageBodyHarness(props: MountStageBodyProps) -> Element {
    build_and_provide_app_context();
    use_live_capture_provider();
    use_editor_state_provider();
    (props.body_fn)()
}

/// Mount a stage-body RSX expression in a test `VirtualDom` with the
/// canonical `AppContext` + `EditorState` + live-capture provider stack.
/// Returns the rendered HTML.
pub(crate) fn mount_stage_body_test(body_fn: fn() -> Element) -> String {
    let mut dom =
        VirtualDom::new_with_props(MountStageBodyHarness, MountStageBodyProps { body_fn });
    dom.rebuild_in_place();
    render(&dom)
}

fn build_and_provide_app_context() {
    use crate::context::{
        ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles, SettingsSnapshot,
    };
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    let (cmd_tx, _rx) = mpsc::channel();
    let raw = RawHandles {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
    };
    use_context_provider(|| raw.clone());
    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    let ctx = AppContext {
        state: Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings,
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_renders_trivial_body() {
        fn body() -> Element {
            rsx! { div { class: "probe", "ok" } }
        }
        let html = mount_stage_body_test(body);
        assert!(html.contains(r#"class="probe""#));
        assert!(html.contains("ok"));
    }
}
