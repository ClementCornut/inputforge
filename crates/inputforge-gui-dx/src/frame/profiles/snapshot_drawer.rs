use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{
    Badge, BadgeVariant, ButtonSize, ButtonVariant, Drawer, DrawerAnchor, DrawerVariant, Icon,
    IconButton,
};
use crate::context::AppContext;
use crate::context::SnapshotRowView;
use crate::frame::profiles::actions::{
    create_manual_snapshot_action, snapshot_delete_action, snapshot_restore_action,
};
use crate::icons::Icon as IconKind;
use inputforge_core::snapshot::SnapshotKind;

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn SnapshotDrawer(
    active_profile_name: String,
    rows: Vec<SnapshotRowView>,
    open: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut drawer_open = use_signal(|| open);
    let count = rows.len();
    let is_empty = rows.is_empty();
    let commands = ctx.commands.clone();
    let snapshot_now = move |_| {
        let _ = commands.send(create_manual_snapshot_action());
    };
    let is_open = *drawer_open.read();
    let toggle = move |_| {
        let next = !*drawer_open.read();
        drawer_open.set(next);
    };
    rsx! {
        section { class: "snapshot-drawer",
            // Always-visible bar: toggle on the left, actions on the right.
            // Sibling of the Drawer so it stays put when the body collapses.
            div { class: "snapshot-drawer__bar",
                button {
                    class: "if-button if-button--ghost if-button--md snapshot-drawer__toggle",
                    id: "snapshot-drawer-bar-title",
                    "aria-expanded": if is_open { "true" } else { "false" },
                    "aria-controls": "snapshot-drawer-body",
                    onclick: toggle,
                    span { class: "snapshot-drawer__chevron",
                        Icon { name: IconKind::ChevronUp }
                    }
                    span { class: "snapshot-drawer__title", "Snapshots · {active_profile_name}" }
                    Badge { variant: BadgeVariant::Info, "{count}" }
                }
                div { class: "snapshot-drawer__actions",
                    IconButton {
                        icon: IconKind::Plus,
                        label: "Snapshot now",
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: snapshot_now,
                    }
                }
            }
            Drawer {
                anchor: DrawerAnchor::Bottom,
                variant: DrawerVariant::Persistent,
                open: is_open,
                aria_labelledby: "snapshot-drawer-bar-title".to_owned(),
                class: "snapshot-drawer__drawer".to_owned(),
                div { id: "snapshot-drawer-body", class: "snapshot-drawer__body",
                    if is_empty {
                        div { class: "snapshot-drawer__empty",
                            span { class: "snapshot-drawer__empty-eyebrow", "No snapshots yet" }
                            span { class: "snapshot-drawer__empty-hint",
                                "Press Ctrl+S or use the + button to capture the current profile state."
                            }
                        }
                    }
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
                            // Map the kind to a single-glyph cue at the row's
                            // leading edge. The icon replaces the prior kind
                            // badge so the strong-weight label can be the row's
                            // anchor without a competing colored chip.
                            let icon_name = match row.kind {
                                SnapshotKind::AutoSessionStart => IconKind::Play,
                                SnapshotKind::AutoBeforeRestore => IconKind::Refresh,
                                SnapshotKind::AutoBeforeBulkMap => IconKind::Link,
                                SnapshotKind::Manual => IconKind::Save,
                            };
                            // The strong-weight label: prefer the engine /
                            // user supplied label, fall back to the kind name
                            // for autos that do not carry a custom label
                            // (Session start, Before restore).
                            let primary_label = row
                                .label
                                .clone()
                                .unwrap_or_else(|| row.kind_label.clone());
                            // Session-start is the recovery anchor for the
                            // current session: deleting it strands the user's
                            // "go back to where I was" lifeline. Disable the
                            // Delete affordance on this row.
                            let session_start =
                                matches!(row.kind, SnapshotKind::AutoSessionStart);
                            // Two-line row layout: the 320px-wide drawer
                            // does not have horizontal real estate for icon
                            // + label + time + 2 action buttons in a single
                            // flex line (the label gets squeezed to ~85px
                            // and ellipses out almost everything). Stacking
                            // label-on-line-1, time + actions-on-line-2 lets
                            // the label take ~280px, restoring the
                            // glanceable read.
                            rsx! {
                                article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                                    span { class: "snapshot-row__kind-icon", "aria-hidden": "true",
                                        Icon { name: icon_name }
                                    }
                                    div { class: "snapshot-row__main",
                                        // Line 1: label + Pinned badge.
                                        // `title` carries the full label text
                                        // so the ellipsis-truncated form
                                        // surfaces a native tooltip on hover.
                                        div { class: "snapshot-row__primary",
                                            strong {
                                                class: "snapshot-row__label",
                                                title: primary_label.clone(),
                                                "{primary_label}"
                                            }
                                            if row.pinned { Badge { variant: BadgeVariant::Success, "Pinned" } }
                                        }
                                        // Line 2: muted time on the left,
                                        // Restore + Delete icons on the
                                        // right. Restore uses
                                        // `clock-counter-clockwise` (Phosphor
                                        // restore-from-history glyph), stays
                                        // always-visible. Delete is the
                                        // ghost trash icon revealed on
                                        // row-hover or focus.
                                        div { class: "snapshot-row__secondary",
                                            time {
                                                class: "snapshot-row__time",
                                                "datetime": row.time_absolute.clone(),
                                                title: row.time_absolute.clone(),
                                                "{row.time_relative}"
                                            }
                                            div { class: "snapshot-row__actions",
                                                IconButton {
                                                    icon: IconKind::ClockCounterClockwise,
                                                    label: "Restore snapshot",
                                                    variant: ButtonVariant::Ghost,
                                                    size: ButtonSize::Sm,
                                                    class: "snapshot-row__restore".to_owned(),
                                                    onclick: restore_click,
                                                }
                                                IconButton {
                                                    icon: IconKind::Trash,
                                                    label: "Delete snapshot",
                                                    variant: ButtonVariant::Ghost,
                                                    size: ButtonSize::Sm,
                                                    class: "snapshot-row__delete".to_owned(),
                                                    disabled: session_start,
                                                    onclick: delete_click,
                                                }
                                            }
                                        }
                                    }
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
