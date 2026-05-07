use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{
    Badge, BadgeVariant, BottomDrawer, Button, ButtonSize, ButtonVariant, IconButton,
};
use crate::context::AppContext;
use crate::context::SnapshotRowView;
use crate::frame::profiles::actions::{
    create_manual_snapshot_action, snapshot_delete_action, snapshot_restore_action,
};
use crate::icons::Icon as IconKind;

#[component]
pub(crate) fn SnapshotDrawer(
    active_profile_name: String,
    rows: Vec<SnapshotRowView>,
    open: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut drawer_open = use_signal(|| open);
    let count = rows.len();
    let commands = ctx.commands.clone();
    let snapshot_now = move |_| {
        let _ = commands.send(create_manual_snapshot_action());
    };
    rsx! {
        section { class: "snapshot-drawer",
            BottomDrawer {
                open: *drawer_open.read(),
                title: format!("Snapshots · {active_profile_name}"),
                count,
                on_toggle: move |_| {
                    let next = !*drawer_open.read();
                    drawer_open.set(next);
                },
                actions: rsx! {
                    IconButton {
                        icon: IconKind::Plus,
                        label: "Snapshot now",
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: snapshot_now,
                    }
                },
                for row in rows {
                    {
                        let commands = ctx.commands.clone();
                        let restore_id = row.id;
                        let restore_click = move |_| {
                            let _ = commands.send(snapshot_restore_action(restore_id).command);
                        };
                        let commands = ctx.commands.clone();
                        let delete_id = row.id;
                        let delete_click = move |_| {
                            let _ = commands.send(snapshot_delete_action(delete_id).command);
                        };
                        rsx! {
                            article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                                div { class: "snapshot-row__main",
                                    div { class: "snapshot-row__title",
                                        Badge { variant: BadgeVariant::Info, "{row.kind_label}" }
                                        if row.pinned { Badge { variant: BadgeVariant::Success, "Pinned" } }
                                        if let Some(label) = &row.label { strong { "{label}" } }
                                    }
                                    span { class: "snapshot-row__time", "{row.time_label}" }
                                }
                                div { class: "snapshot-row__actions",
                                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, onclick: restore_click, "Restore" }
                                    Button { variant: ButtonVariant::Danger, size: ButtonSize::Sm, onclick: delete_click, "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusScope {
    Panel,
    TextInput,
    InlineRename,
    Menu,
    Dialog,
    /// Reserved for the focus state right after an OS file picker closes.
    /// Spec line 312 calls it out as a gating context, but the picker is
    /// OS-modal: while the spawn task awaits `pick_file().await`, no
    /// keydown reaches the `WebView` listener at all, so detection from the
    /// DOM is unnecessary and this variant is never produced from
    /// [`classify_focus_scope`]. Kept in the enum so the spec table maps
    /// 1:1 to `FocusScope` and the suppression test still covers it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "spec-mandated focus scope; only constructed in unit tests \
                      because OS-modal pickers block the WebView keydown loop, \
                      so no keydown event can ever observe this scope at runtime"
        )
    )]
    OsPickerReturn,
}

pub(crate) fn should_handle_snapshot_shortcut(scope: FocusScope) -> bool {
    matches!(scope, FocusScope::Panel)
}

/// Snapshot of the focused element, populated from the `WebView` keydown
/// handler before being routed to [`classify_focus_scope`].
///
/// All boolean fields are derived from `document.activeElement`'s tag
/// and ancestor chain in JS, the Rust side stays a pure mapper for
/// straightforward unit testing.
///
/// Each bool maps 1:1 to one of the gating contexts in the spec table
/// (text input / inline rename / menu / dialog). A state machine would
/// hide the parallel structure with both the JS classifier and the
/// resulting [`FocusScope`] arms, so we override
/// `clippy::struct_excessive_bools`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool maps 1:1 to a gating context in the spec table; \
              the parallel structure is the point and a state machine would \
              obscure it"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FocusContext<'a> {
    /// `tagName` of the focused element (uppercase per the DOM spec, but
    /// matched case-insensitively here).
    pub tag: &'a str,
    /// `HTMLElement.isContentEditable` on the focused element.
    pub is_content_editable: bool,
    /// `closest('.profile-row__rename')` matched a non-null ancestor.
    pub in_inline_rename: bool,
    /// `closest('[role="menu"]')` matched a non-`aria-hidden="true"` menu.
    pub in_menu: bool,
    /// `closest('[role="dialog"][aria-modal="true"]')` matched, or any
    /// open `<dialog>` is present in the document.
    pub in_dialog: bool,
}

/// Map a focus snapshot to the corresponding [`FocusScope`].
///
/// Precedence mirrors the spec's gating table: Dialog and Menu trump the
/// inline-rename check, which itself trumps a generic text-input match.
/// Anything that doesn't trip one of these gates is treated as `Panel`,
/// the only scope that allows the manual-snapshot shortcut to fire.
pub(crate) fn classify_focus_scope(ctx: FocusContext<'_>) -> FocusScope {
    if ctx.in_dialog {
        return FocusScope::Dialog;
    }
    if ctx.in_menu {
        return FocusScope::Menu;
    }
    if ctx.in_inline_rename {
        return FocusScope::InlineRename;
    }
    if ctx.tag.eq_ignore_ascii_case("INPUT")
        || ctx.tag.eq_ignore_ascii_case("TEXTAREA")
        || ctx.is_content_editable
    {
        return FocusScope::TextInput;
    }
    FocusScope::Panel
}

/// Window-level keydown listener that watches for `Ctrl+S` (or `Cmd+S`
/// on macOS) and, when focus is outside editable / modal UI, dispatches
/// [`EngineCommand::CreateSnapshot`] for a manual snapshot of the
/// active profile.
///
/// The JS side is the source of truth for focus-scope classification:
/// it inspects `document.activeElement` and forwards a tuple
/// `(tag, content_editable, in_inline_rename, in_menu, in_dialog)` to
/// Rust, which routes through [`classify_focus_scope`] +
/// [`should_handle_snapshot_shortcut`]. When the gate denies the
/// shortcut, the JS side does NOT call `preventDefault`, so the browser
/// default save (a no-op inside `WebView2` for our app) is undisturbed
/// and the keystroke remains available to any inner handler that may
/// want it. When the gate allows the shortcut, JS preempts the default
/// and forwards a single-shot signal that this loop translates into a
/// snapshot command.
///
/// Mirrors the document-level listener pattern used by the mapping rail
/// (see `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`): one
/// install per app-root mount via [`use_hook`], a `dioxus.send` /
/// `dioxus.recv` pair for cross-boundary messaging, and capture-phase
/// registration so the listener fires before any element-level handler.
pub(crate) fn install_snapshot_shortcut_listener(commands: Sender<EngineCommand>) {
    use_hook(move || {
        spawn(async move {
            let mut handle = document::eval(SHORTCUT_LISTENER_JS);

            loop {
                let Ok((tag, content_editable, in_inline_rename, in_menu, in_dialog)) =
                    handle.recv::<(String, u8, u8, u8, u8)>().await
                else {
                    break;
                };

                let ctx = FocusContext {
                    tag: tag.as_str(),
                    is_content_editable: content_editable == 1,
                    in_inline_rename: in_inline_rename == 1,
                    in_menu: in_menu == 1,
                    in_dialog: in_dialog == 1,
                };
                let scope = classify_focus_scope(ctx);
                if !should_handle_snapshot_shortcut(scope) {
                    tracing::debug!(
                        target: "f13::profiles",
                        action = "snapshot_shortcut_suppressed",
                        ?scope,
                        "Ctrl+S suppressed inside editable or modal context",
                    );
                    continue;
                }

                tracing::info!(
                    target: "f13::profiles",
                    action = "snapshot_shortcut_fired",
                    "dispatch CreateSnapshot (manual, no label)",
                );
                if let Err(err) = commands.send(create_manual_snapshot_action()) {
                    tracing::warn!(
                        target: "f13::profiles",
                        action = "snapshot_shortcut_dispatch_failed",
                        error = %err,
                        "manual snapshot dispatch failed",
                    );
                }
            }
        });
    });
}

/// JS side of the Ctrl+S listener. Capture-phase window keydown that
/// matches `Ctrl+S` / `Cmd+S` (any `Shift` variant is rejected, plain
/// `S` and other letters are ignored), inspects the focused element to
/// derive the focus scope, and forwards a 5-tuple to the Rust loop.
///
/// `preventDefault` is only called on the gated allow path so the
/// browser default for editable contexts (text input shortcuts, etc.)
/// is preserved. The browser default `Save Page` action is a no-op
/// inside `WebView2` for our app.
const SHORTCUT_LISTENER_JS: &str = "\
const h = (ev) => {\n\
   if (ev.key !== 's' && ev.key !== 'S') return;\n\
   if (!(ev.ctrlKey || ev.metaKey)) return;\n\
   if (ev.shiftKey || ev.altKey) return;\n\
   const ae = document.activeElement;\n\
   const tag = ae && ae.tagName ? ae.tagName : '';\n\
   const ce  = ae && ae.isContentEditable ? 1 : 0;\n\
   const rn  = ae && ae.closest && ae.closest('.profile-row__rename') ? 1 : 0;\n\
   const mn  = ae && ae.closest && ae.closest('[role=\"menu\"]:not([aria-hidden=\"true\"])') ? 1 : 0;\n\
   let dlg   = ae && ae.closest && ae.closest('[role=\"dialog\"][aria-modal=\"true\"]') ? 1 : 0;\n\
   if (!dlg && document.querySelector('dialog[open]')) dlg = 1;\n\
   const editable = (tag === 'INPUT' || tag === 'TEXTAREA' || ce === 1);\n\
   const gated = (dlg === 1 || mn === 1 || rn === 1 || editable);\n\
   if (!gated) ev.preventDefault();\n\
   dioxus.send([tag, ce, rn, mn, dlg]);\n\
};\n\
window.addEventListener('keydown', h, true);\n\
(async () => { while (true) { await dioxus.recv(); } })();\n\
";
