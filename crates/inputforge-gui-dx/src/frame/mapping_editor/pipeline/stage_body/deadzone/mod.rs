// Rust guideline compliant 2026-05-03

//! F11 deadzone body.

pub(crate) mod interaction;
pub(crate) mod keyboard;
pub(crate) mod mutation;
pub(crate) mod rendering;
pub(crate) mod state;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::pipeline::stage_body::instruments;
use crate::frame::mapping_editor::undo_log::StageId;

/// Project the deadzone config stored at `stage_id` from the current root
/// `actions`. Falls back to `fallback` when projection fails (transient
/// mid-edit state before the dispatcher writes the new action tree).
fn project_stage_config(
    actions: &[Action],
    stage_id: &StageId,
    fallback: &DeadzoneConfig,
) -> DeadzoneConfig {
    match at_path(actions, stage_id) {
        Some(Action::Deadzone { config }) => config.clone(),
        _ => fallback.clone(),
    }
}

/// Single-message dispatcher invoked by the eval loop in `on_mounted`. Routes
/// each event kind through the pure handlers in `interaction.rs` and updates
/// the body component's signals in place. All `Signal<T>` arguments are
/// captured by `Copy`; the heavier values (`mapping_key`, `stage_id`, etc.)
/// are borrowed because they live in the spawn task's stack across iterations.
#[expect(
    clippy::too_many_arguments,
    reason = "Body-shaped dispatch keeps each per-event handler local."
)]
fn dispatch_bridge_event(
    payload: &instruments::bridge::BridgeEvent,
    mut body: Signal<state::BodyState>,
    mut working_config: Signal<Option<DeadzoneConfig>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
    mut undo_log: Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    mut malformed_hints: Signal<std::collections::HashMap<StageId, String>>,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    config_seed: &DeadzoneConfig,
    cmd_tx: &std::sync::mpsc::Sender<inputforge_core::engine::EngineCommand>,
) {
    if payload.rs <= 0.0 {
        return;
    }
    let rect = interaction::PlotRect {
        x: payload.rl,
        y: payload.rt,
        size: payload.rs,
    };

    match payload.kind.as_str() {
        "down" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (next, _, _) =
                interaction::handle_pointer_down(prev, &live, (payload.x, payload.y), &rect);
            if next.dragging.is_some() {
                working_config.set(Some(live));
            }
            body.set(next);
        }
        "move" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (next, new_cfg_opt, _) =
                interaction::handle_pointer_move(prev, &live, (payload.x, payload.y), &rect);
            if let Some(new_cfg) = new_cfg_opt {
                working_config.set(Some(new_cfg));
            }
            body.set(next);
        }
        "up" => {
            let prev = body.peek().clone();
            // Run the up handler when there is anything to clear: a real
            // drag, OR a deferred stacked-center pending_split that the
            // user clicked into without moving (no dispatch in that case;
            // the phantom-undo guard below catches it).
            if prev.dragging.is_none() && prev.pending_split.is_none() {
                return;
            }
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            let actions_snap = actions.to_vec();
            let name = cfg.mapping_names.get(&mapping_key.1).cloned();
            drop(cfg);
            let dragged = working_config
                .peek()
                .clone()
                .unwrap_or_else(|| live.clone());
            working_config.set(None);
            let (mut next, result, _) = interaction::handle_pointer_up(prev, &dragged);
            // Phantom-undo guard: a mousedown + mouseup with no intervening
            // move means `working_config` was never updated, so `dragged ==
            // live`. Without this guard the body would dispatch a no-op
            // `SetMapping` and record a `"deadzone: drag"` undo entry for a
            // user gesture that did not change anything. F10 has the
            // identical bug today; Task 17 backports the same guard.
            if dragged == live {
                body.set(next);
                return;
            }
            match result {
                Ok(valid) => {
                    instruments::stage_dispatch::dispatch_stage_edit(
                        &actions_snap,
                        stage_id,
                        Action::Deadzone { config: valid },
                        mapping_key,
                        name,
                        cmd_tx,
                        &mut undo_log,
                        "deadzone: drag".to_owned(),
                    );
                    malformed_hints.write().remove(stage_id);
                }
                Err(err) if err.is_empty() => {}
                Err(err) => {
                    let _revert = next.pre_drag_config.take();
                    malformed_hints.write().insert(stage_id.clone(), err);
                }
            }
            body.set(next);
        }
        // dbl, ctx ignored: F11 has no double-click or right-click semantics.
        _ => {}
    }
}

/// Body component for an `Action::Deadzone` pipeline stage.
///
/// Renders the four-field numeric toolbar and the SVG plot with full pointer
/// interaction: drag the four threshold handles along the X axis. Dispatches
/// `SetMapping` and pushes undo entries on drag-end (when the dragged config
/// differs from the live one).
///
/// The `config` and `root_actions` props are first-render seeds only. The
/// live source of truth is `ConfigSnapshot.selected_mapping_actions` read
/// from the `AppContext` signal so that undo replay, external edits, and
/// sibling-stage mutations all propagate to this component without a prop
/// change.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn DeadzoneBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    config: DeadzoneConfig,
    /// Outermost actions vec for the mapping, threaded by F9's `StageBody`.
    /// Used as the initial-render seed; the live source is
    /// `ConfigSnapshot.selected_mapping_actions` from context.
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let config_signal = ctx.config;
    let editor = use_context::<EditorState>();
    let mut undo_log = editor.undo_log;
    let malformed_hints = editor.malformed_hints;

    let mut body: Signal<state::BodyState> = use_signal(state::BodyState::default);

    // `working_config` tracks the in-flight drag config so `on_pointer_up`
    // can commit the final dragged values. `None` when no drag is active and
    // `Some(c)` during a drag (updated on every `on_pointer_move` that
    // returns a new local config). Reset to `None` on pointer-up.
    let working_config: Signal<Option<DeadzoneConfig>> = use_signal(|| None);

    // Captured once per mount so that `now_ms` inside `on_key` can be
    // computed as `Instant::now().saturating_duration_since(*time_baseline.peek())`.
    // No `Instant::EPOCH` exists on either std or web_time, so a baseline
    // captured here is the simplest source of monotonic ms.
    let time_baseline = use_signal(std::time::Instant::now);

    // Re-project on each render so the toolbar and plot see the freshest
    // live data. Clone the snapshot to drop the read guard before later
    // reads.
    let cfg = config_signal.read().clone();
    let live_actions = cfg
        .selected_mapping_actions
        .clone()
        .unwrap_or_else(|| root_actions.clone());
    let live_config = project_stage_config(&live_actions, &stage_id, &config);

    // Live tracking dot via the shared instrument helper. Same gates as
    // F10: top-level stage, bound input, connected device, axis input.
    let live_value: Option<f64> = instruments::live_axis::compute_live_axis_value(
        &stage_id,
        &mapping_key.1,
        &ctx,
        &live_actions,
    );

    // ----- JS-bridge mouse-event dispatch -----
    //
    // Same workaround as F10: Dioxus 0.7.6 desktop's delegated event
    // dispatcher does NOT route mousedown / mousemove / mouseup to handlers
    // on a non-button `<div>`, so the shared `instruments::bridge` module
    // installs raw document-level listeners via `document::eval` and routes
    // events back through an event channel into `dispatch_bridge_event`.
    let plot_dom_id = instruments::bridge::stage_id_dom_id("if-deadzone-plot", &stage_id);
    let mapping_key_for_bridge = mapping_key.clone();
    let stage_id_for_bridge = stage_id.clone();
    let cmd_tx_for_bridge = ctx.commands.clone();
    let config_seed_for_bridge = config.clone();
    let dispatch = move |payload: instruments::bridge::BridgeEvent| {
        dispatch_bridge_event(
            &payload,
            body,
            working_config,
            config_signal,
            undo_log,
            malformed_hints,
            &mapping_key_for_bridge,
            &stage_id_for_bridge,
            &config_seed_for_bridge,
            &cmd_tx_for_bridge,
        );
    };
    let on_mounted = instruments::bridge::mount_mouse_bridge(plot_dom_id.clone(), dispatch);

    // ----- Keyboard -----
    // Routes a normalized `KeyInput` through `keyboard::handle_key`,
    // updates body state, and dispatches the resulting config edit (if
    // any) to the engine. Tab / ShiftTab do NOT call `prevent_default`
    // so the browser can advance focus past the plot at the boundary;
    // all other handled keys consume the event.
    let mapping_key_for_key = mapping_key.clone();
    let stage_id_for_key = stage_id.clone();
    let cmd_tx_for_key = ctx.commands.clone();
    let config_seed_for_key = config.clone();
    let on_key = move |evt: KeyboardEvent| {
        let key = match (evt.key(), evt.modifiers().shift()) {
            (Key::Tab, true) => keyboard::KeyInput::ShiftTab,
            (Key::Tab, false) => keyboard::KeyInput::Tab,
            (Key::ArrowLeft, shift) => keyboard::KeyInput::ArrowLeft { shift },
            (Key::ArrowRight, shift) => keyboard::KeyInput::ArrowRight { shift },
            (Key::ArrowUp, shift) => keyboard::KeyInput::ArrowUp { shift },
            (Key::ArrowDown, shift) => keyboard::KeyInput::ArrowDown { shift },
            (Key::Home, _) => keyboard::KeyInput::Home,
            (Key::End, _) => keyboard::KeyInput::End,
            (Key::Enter, _) => keyboard::KeyInput::Enter,
            (Key::Delete | Key::Backspace, _) => keyboard::KeyInput::Delete,
            (Key::Escape, _) => keyboard::KeyInput::Escape,
            _ => return,
        };
        if !matches!(key, keyboard::KeyInput::Tab | keyboard::KeyInput::ShiftTab) {
            evt.prevent_default();
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "ms-since-mount fits u64 for any session"
        )]
        let now_ms = std::time::Instant::now()
            .saturating_duration_since(*time_baseline.peek())
            .as_millis() as u64;
        let cfg = config_signal.read();
        let actions: Vec<Action> = cfg.selected_mapping_actions.clone().unwrap_or_default();
        let live_cfg = project_stage_config(&actions, &stage_id_for_key, &config_seed_for_key);
        let name = cfg.mapping_names.get(&mapping_key_for_key.1).cloned();
        drop(cfg);
        let (next, new_cfg, outcome, _) =
            keyboard::handle_key(body.peek().clone(), &live_cfg, key, now_ms);
        body.set(next);
        let Some(new) = new_cfg else { return };
        match outcome {
            Some(keyboard::KeyOutcome::PushUndo { label }) => {
                instruments::stage_dispatch::dispatch_stage_edit(
                    &actions,
                    &stage_id_for_key,
                    Action::Deadzone { config: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                    &mut undo_log,
                    label,
                );
            }
            Some(keyboard::KeyOutcome::MergeUndo) => {
                instruments::stage_dispatch::dispatch_stage_edit_no_undo(
                    &actions,
                    &stage_id_for_key,
                    Action::Deadzone { config: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                );
            }
            None => {} // Escape revert: body-local, no dispatch.
        }
    };

    // Reset coalesce on focus-out so the next nudge after refocus pushes a
    // fresh undo entry rather than merging into a stale prior burst.
    let mut body_for_focusout = body;
    let on_focus_out = move |_| {
        body_for_focusout.with_mut(|s| {
            s.nudge_coalesce.reset();
        });
    };

    let snap = body.read();
    let dragging_attr = snap.dragging.is_some().to_string();
    let hovered_attr = snap.hovered_handle.is_some().to_string();
    let is_dragging = snap.dragging.is_some();
    drop(snap);

    // While a handle is mid-drag, prefer the in-flight `working_config` so the
    // plot and toolbar fields update synchronously with the cursor instead of
    // waiting for the engine SetMapping round-trip on mouseup. F10 hides this
    // by maintaining a `cached_path` updated in the move arm; F11 renders
    // straight from the config so it needs the live working value here.
    let display_config = if is_dragging {
        working_config
            .read()
            .clone()
            .unwrap_or_else(|| live_config.clone())
    } else {
        live_config.clone()
    };

    rsx! {
        div { class: "if-deadzone",
            toolbar::Toolbar {
                config: display_config.clone(),
                stage_id: stage_id.clone(),
                root_actions: live_actions.clone(),
                mapping_key: mapping_key.clone(),
            }
            div {
                class: "if-deadzone__plot-frame",
                id: "{plot_dom_id}",
                tabindex: "0",
                "aria-label": "deadzone curve",
                "data-hovered": "{hovered_attr}",
                "data-dragging": "{dragging_attr}",
                onmounted: on_mounted,
                onkeydown: on_key,
                onfocusout: on_focus_out,
                { rendering::render_plot(&display_config, &body.read(), live_value) }
            }
        }
    }
}
