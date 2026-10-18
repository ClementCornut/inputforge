//! Editor header: identity row (h2 title with inline rename) plus subtitle
//! row (source label, rebind button, optional output label).
//!
//! Per DESIGN.md §6 "Inline-First Rule": the property is edited where it
//! is displayed. The h2 is both the display surface and (via F2 / right
//! click) the entry point to the inline rename editor. The subtitle row
//! anchors the rebind affordance next to the source label so that source
//! and its action live as one unit.
//!
//! Display state: bare `<h2>` with `tabindex="0"` and `data-editor-focus`
//! so the F8 keyboard nav lands here. `Key::F2` and right-click both arm
//! the rename editor. Long names truncate via CSS `text-overflow: ellipsis`;
//! the user reads the full name by entering rename mode (the inline editor
//! scrolls horizontally).
//!
//! Edit state: `<input type="text">` styled to inherit h2 typography
//! (20px / 600 / Inter, line-height 28px) so the swap is visually
//! invisible. Enter / blur commit, Esc reverts. Empty / whitespace-only
//! inputs revert without dispatch. The undo entry is pushed only when
//! dispatch succeeds, mirroring the F2 commit-on-blur model.
use dioxus::prelude::*;
use inputforge_core::action::{Action, Mapping};
use inputforge_core::types::InputAddress;
#[path = "header_labels.rs"]
mod labels;
use crate::capture::{CaptureFilter, Target};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CAPTURE_PROMPT, use_capture};
use labels::first_map_to_vjoy_label;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures. Mirrors the suppression \
              used in mapping_list/row.rs and mode_tabs/rename_inline.rs."
)]
pub(crate) fn Header(
    /// Current saved mapping name (read-only mirror; edit working copy is local).
    name: String,
    /// Mapping's input address; drives the source-label readout.
    input: InputAddress,
    /// `(mode, InputAddress)` key for rename + rebind dispatch.
    /// Named `mapping_key` to avoid Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// Full action list for the mapping; needed to reconstruct `SetMapping`.
    actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::use_origin();
    let editor = use_context::<EditorState>();
    let armed: Signal<bool> = use_signal(|| false);
    let saved_name = ctx
        .config
        .read()
        .mappings
        .iter()
        .find(|m| m.input == mapping_key.1 && m.mode == mapping_key.0)
        .and_then(|m| m.name.clone());
    let stored_name = saved_name.clone().unwrap_or_default();
    let local_name: Signal<String> = use_signal(|| stored_name.clone());
    let target = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: saved_name.clone(),
        actions: actions.clone(),
    };
    let draft = use_signal(|| target.clone());
    let mapping_key_for_capture = mapping_key.clone();
    let actions_for_capture = actions.clone();
    let cfg_for_capture = ctx.config;
    let name_for_capture = saved_name.clone();
    let capture = use_capture(
        Target {
            profile: ctx.meta.read().profile_id.clone(),
            mode: mapping_key.0.clone(),
            mapping: Some(mapping_key.1.clone()),
            name: ctx
                .config
                .read()
                .mappings
                .iter()
                .find(|mapping| mapping.input == mapping_key.1 && mapping.mode == mapping_key.0)
                .and_then(|mapping| mapping.name.clone()),
            actions: Some(actions.clone()),
            field: "primary".into(),
        },
        CaptureFilter::Any,
        move |new_addr| {
            let (_, old_addr) = mapping_key_for_capture.clone();
            let old_label = source_label::format(&old_addr, &cfg_for_capture.read());
            let new_label = source_label::format(&new_addr, &cfg_for_capture.read());
            let label = format_undo_label(
                UndoKind::Rebind,
                LabelArgs {
                    old_new: Some((&old_label, &new_label)),
                    ..LabelArgs::default()
                },
            );
            let _ = super::pipeline::stage_body::instruments::stage_dispatch::submit_inline(
                &mapping_key_for_capture,
                &actions_for_capture,
                name_for_capture.clone(),
                crate::mapping_edits::Edit::Rebind(new_addr),
                label,
                false,
            );
        },
        |()| {},
    );
    let cfg = ctx.config.read();
    let src = source_label::format(&input, &cfg);
    let output_label = cfg
        .selected_mapping_actions
        .as_ref()
        .and_then(|a| first_map_to_vjoy_label(a));
    drop(cfg);
    let ui = use_context::<crate::mapping_edits::Bindings>().ui;
    let publication = crate::mapping_edits::ui::Header {
        armed,
        value: local_name,
        draft,
        error: editor.error,
    };
    let profile = super::pipeline::stage_body::instruments::stage_dispatch::profile();
    let send = use_callback(move |event| {
        ui.header.call(crate::mapping_edits::ui::HeaderRequest {
            publication,
            profile: profile.clone(),
            target: target.clone(),
            event,
        });
    });
    let on_h2_keydown = move |evt| send.call(crate::mapping_edits::ui::HeaderEvent::ArmKey(evt));
    let on_h2_contextmenu =
        move |evt| send.call(crate::mapping_edits::ui::HeaderEvent::ArmMouse(evt));
    let on_input_keydown = move |evt| send.call(crate::mapping_edits::ui::HeaderEvent::Key(evt));
    let on_input_blur = move |_| send.call(crate::mapping_edits::ui::HeaderEvent::Blur);
    let on_input_input = use_callback(move |evt: FormEvent| {
        send.call(crate::mapping_edits::ui::HeaderEvent::Input(evt.value()));
    });
    let on_rebind = move |_: MouseEvent| capture.start.call(());
    let on_cancel_rebind = move |_: MouseEvent| capture.cancel.call(());
    let is_rebind_listening = *capture.listening.read();
    rsx! {
        div {
            class: "if-editor__header",
            if *armed.read() {
                crate::components::InlineTextEditor {
                    value: local_name,
                    variant: crate
                            ::components::TextInputVariant::Title,
                    class: "if-editor__title-input",
                    aria_label: "Edit mapping name",
                    id: "mapping-title",
                    aria_describedby: editor.error.read().as_ref().map(|_| "mapping-editor-error".to_owned()),
                    data_editor_focus: "true",
                    oninput: on_input_input,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                            let _ = document::eval(
                                r"
                                                const el = document.activeElement;
                                                if (el && typeof el.select === 'function') { el.select(); }
                                                ",
                            );
                        });
                    },
                }
            } else {
                h2 {
                    class: "if-editor__title",
                    tabindex: "0",
                    "data-editor-focus": "true",
                    "aria-label": "Mapping name. Press F2 or right-click to rename.",
                    onkeydown: on_h2_keydown,
                    oncontextmenu: on_h2_contextmenu,
                    "{name}"
                }
            }
            div {
                class: "if-editor__subtitle",
                crate::components::RebindField {
                    source_label: src,
                    listening: is_rebind_listening,
                    unbound: matches!(input, InputAddress::Unbound),
                    prompt: CAPTURE_PROMPT,
                    action_label: "Rebind mapping input",
                    cancel_label: "Cancel mapping input capture",
                    onclick: on_rebind,
                    oncancel: on_cancel_rebind,
                }
                if let Some(out) = output_label {
                    span {
                        class: "if-editor__subtitle-arrow",
                        "\u{00a0}\u{00a0}\u{2192}\u{00a0}\u{00a0}"
                    }
                    span {
                        class: "if-editor__subtitle-output",
                        "{out}"
                    }
                }
            }
        }
    }
}
