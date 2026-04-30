# F7, Application Frame: Top Bar, Banner, Status Bar, Panel Slot, Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-29
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Core Screens feature F7 (post-F5 rewrite)
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system), [F3](./2026-04-26-f3-app-shell-tray-bridge-design.md) (placeholder shell + tray), [F4](./2026-04-26-f4-toast-dialog-design.md) (toast + dialog), [F5](./2026-04-27-f5-architecture-ia-redesign-design.md) (IA redesign), [F6](./2026-04-28-f6-snapshot-preferences-core-design.md) (snapshot module + `mode_force` + preferences)
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)

---

## Context

F7 is the chrome-shell upgrade, the third foundation feature after F5's IA redesign and F6's snapshot/preferences/`mode_force` core work. F7 replaces F3's disposable `PlaceholderShell` with the real F5 layout:

- a top bar carrying the engine pill, profile name, mode tabs, and the secondary tools cluster;
- a conditional divergence/forced-mode banner sitting between the top bar and the main row;
- a thin status bar at the bottom with three glance-only slots (warnings · device count · profile path);
- a right-side panel slot that F12 (Devices) and F13 (Profiles) plug into.

F7 is GUI-side with one core-side bolt-on (four new mode-CRUD `EngineCommand` variants, Add / Rename / Delete / SetDefault). It owns no engine state. It reads `MetaSnapshot` (extended with three new fields) and writes a new GUI-only `ViewState` context that downstream features (F8, F9, F12, F13) consume.

The egui GUI stays the default runtime behavior. F7 changes are scoped under `#[cfg(feature = "gui-dioxus")]` boundaries; the egui code paths stay byte-identical.

---

## Confirmed design choices

Decisions made during brainstorming that shape this spec:

1. **Single `ViewState` context** holds GUI-only chrome state. Three Signals: `editing_mode: String`, `panel_slot: PanelSlot`, `via_calibration: bool`. Provided in `app_root` alongside the existing `AppContext`, `LaunchParams`, and `ToastQueue`. Module root is `frame/`, names "shell" and "chrome" rejected as opaque.

2. **`PanelSlot` is a flat tri-state enum**, `None | Devices | Profiles`. `via_calibration` is a separate Signal, sticky while `panel_slot == Devices`. F12 reads `via_calibration` on mount to pick its initial drill target; F7 doesn't reach into Devices internals.

3. **Mode CRUD adds four granular `EngineCommand` variants:** `AddMode { name, parent }`, `RenameMode { from, to }`, `DeleteMode { name }`, `SetDefaultMode { name }`. Engine handlers atomically mutate the active profile's `ModeTree` / `ProfileSettings::startup_mode` and persist via the existing `Profile::save` path. Delete is **cascade**, mappings scoped to the deleted mode are deleted with it; F4 destructive dialog enumerates the affected count.

4. **Editing-mode initial value = `profile.settings.startup_mode`** at every profile load/switch. Sticky-per-profile persistence is a deferred F15 enhancement on top of `Preferences`, not F7. Deletion of the active editing-mode tab falls back to `startup_mode`; rename updates the active editing-mode in place.

5. **Banner state machine** drives off `(editing_mode, current_mode, mode_force)` only, engine status doesn't gate visibility (Activate-while-paused is "activate-and-resume" in one click per F5). Three visible states (Diverged · Forced aligned · Forced+Diverged), each with single-line copy and one-or-two buttons. ARIA: `role="status"` + `aria-live="polite"`. Color tokens: control-violet for pure Diverged, warning-amber for any Forced state.

6. **Inline editing for Add/Rename modes** uses F2's `TextInput` primitive in `invalid` state. Reject + inline error on commit; blur with invalid value reverts. Trim before commit; non-empty + unique-within-profile validation. Esc cancels; Enter (or blur with valid) commits.

7. **`MetaSnapshot` gains three fields**, `mode_force: Option<ForcedMode>`, `modes: Vec<String>`, `startup_mode: Option<String>`. Polling-task projection extends `MetaSnapshot::from_state` only; `bridge.rs` is structurally unchanged. `PartialEq` gating preserves the unchanged-snapshot no-op contract.

8. **Render discipline:** every component reads through narrow `use_memo` slices over its dependencies. All non-render logic (status→variant mapping, banner state derivation, count derivation, divergence detection, runtime-marker placement, name validation) lives in pure `logic.rs` files next to each region's `mod.rs`. No Signal reads inside `logic.rs`. Steady-state idle target: zero F7 re-renders per polling tick.

9. **File structure** rooted at `frame/`. Each region is a folder; `mod.rs` is render, sibling `logic.rs` is pure. Sub-components (`engine_pill/`, `mode_tabs/`, `tools_cluster/`) split into folders when non-trivial. CSS assets mirror under `assets/frame/`.

10. **Calibration sugar routing** lives in `tools_cluster/logic.rs`. Clicking Calibration sets `panel_slot = Devices, via_calibration = true`; clicking Devices sets `panel_slot = Devices, via_calibration = false`. F12 reads `via_calibration` on mount. Top-bar Devices/Calibration items are disabled when `meta.profile_name.is_none()`; Profiles is always enabled.

## Non-goals (out of scope for this spec)

- **Pixel-level visual treatment** of any surface. `impeccable:frontend-design` is invoked during F7 implementation, not during this brainstorm. F5's `chrome-a-refined.html` is the visual reference baseline.
- **Mode-tree visualization for many-mode profiles.** F14 owns this if implementation discovers it warranted.
- **Force-mode keyboard shortcut** (parent-plan open question). Right-click menu only in F7.
- **Push-based engine→GUI updates.** Deferred to a post-F17 dedicated feature; cost-of-change analysis recorded below in *Open questions*.
- **Sticky editing-mode persistence.** Promoted to F15/Preferences if needed.
- **Light theme.** Out of scope for the rewrite per parent plan.
- **Per-window close behavior beyond F3.** F7 doesn't introduce new windows.

---

## Architecture

### File structure under `crates/inputforge-gui-dx/src/`

```
src/
├── frame/                       # NEW, root for everything F7 owns
│   ├── mod.rs                   #   pub(crate) re-exports: Layout, ViewState, ViewStateProvider
│   ├── view_state.rs            #   ViewState struct + PanelSlot enum + provider hook
│   │
│   ├── layout/
│   │   ├── mod.rs               #   Layout component: main-row OR empty_state branch on profile_name.is_some()
│   │   └── empty_state.rs       #   no-profile fallback (F7 stub; F13 replaces with the real workspace empty state)
│   │
│   ├── top_bar/
│   │   ├── mod.rs               #   horizontal composition: pill | name | tabs | cluster
│   │   ├── engine_pill/
│   │   │   ├── mod.rs           #   render: <button role="status" aria-live="polite">
│   │   │   └── logic.rs         #   pure: engine_pill_state(status, has_profile)
│   │   ├── profile_name.rs      #   single file: <span>/<button> + click → panel_slot = Profiles
│   │   ├── mode_tabs/
│   │   │   ├── mod.rs           #   render: tablist + runtime-marker + + tab + inline editor mounting
│   │   │   ├── logic.rs         #   pure: validate_mode_name, runtime_marker, divergence
│   │   │   ├── context_menu.rs  #   right-click / Shift+F10 menu (F2 Menu primitive consumer)
│   │   │   ├── add_inline.rs    #   `+` inline-expand TextInput → AddMode dispatch
│   │   │   └── rename_inline.rs #   in-tab TextInput → RenameMode dispatch (shares logic.rs)
│   │   └── tools_cluster/
│   │       ├── mod.rs           #   render: Devices · Calibration · Profiles buttons
│   │       └── logic.rs         #   pure: tool_active, panel_slot transitions
│   │
│   ├── banner/
│   │   ├── mod.rs               #   render: conditional shell + copy + buttons
│   │   └── logic.rs             #   pure: derive_banner_state -> BannerState
│   │
│   ├── status_bar/
│   │   ├── mod.rs               #   render: 3-slot composition using components::StatusBar primitive
│   │   └── logic.rs             #   pure: device_count_label, warning_count_label, truncate_path
│   │
│   └── panel_slot/
│       └── mod.rs               #   single file: mounts F12-stub or F13-stub per panel_slot
│
├── components/                  # UNCHANGED, F2/F3 primitives stay
├── theme/                       # UNCHANGED, F2
├── icons/                       # UNCHANGED, F2
├── toast/                       # UNCHANGED, F4
├── tray/                        # UNCHANGED, F3
├── lifecycle/                   # UNCHANGED, F3
├── bridge.rs                    # UNCHANGED structure (projection extended via MetaSnapshot::from_state)
├── context.rs                   # MODIFIED, MetaSnapshot definition + projection (3 new fields)
│
├── shell/                       # DELETED (placeholder.rs + status_bar_view.rs + mod.rs)
├── assets/shell/                # DELETED (placeholder-shell.css)
│
├── app.rs                       # MODIFIED, install ViewState provider; render frame::Layout
└── lib.rs                       # MODIFIED, `mod frame;` replaces `mod shell;`
```

CSS asset tree mirrors:

```
assets/frame/
├── layout.css
├── empty_state.css
├── top_bar.css
├── banner.css
├── status_bar.css
└── panel_slot.css
```

`engine_pill`, `profile_name`, `mode_tabs`, `tools_cluster` all share `top_bar.css` (one stylesheet for the whole top bar, keeps spacing rhythm coherent).

### `ViewState` (`frame/view_state.rs`)

```rust
use dioxus::prelude::*;

/// GUI-only chrome state, provided in `app_root` alongside `AppContext`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewState {
    pub editing_mode:    Signal<String>,
    pub panel_slot:      Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PanelSlot {
    #[default]
    None,
    Devices,
    Profiles,
}

/// Hook installed once in `app_root`. Returns a `ViewState` ready to be passed to
/// `use_context_provider`. Signals are created inside the runtime per Dioxus
/// rules (`Signal::new` outside a hook leaks).
///
/// Initial values:
/// - `editing_mode`    = `meta.startup_mode.clone().unwrap_or_else(|| "Default".to_owned())`
/// - `panel_slot`      = `None`
/// - `via_calibration` = `false`
///
/// One `use_effect` reconciles `editing_mode` against `meta`:
/// - If `meta.profile_name` flips (profile load / switch), reset `editing_mode`
///   to the new `startup_mode`.
/// - Else if `editing_mode` is no longer present in `meta.modes` (mid-session
///   DeleteMode, RestoreSnapshot, or external TOML edit), reset `editing_mode`
///   to the current `startup_mode`.
/// - If `startup_mode` is also missing from `modes` (broken profile), fall
///   back to `modes[0]` (the ModeTree root, since `all_modes()` is DFS-ordered).
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState { … }
```

`use_effect` ordering: Dioxus effects run after render, so the next render after a profile load sees the reset `editing_mode`. The first frame after profile load may briefly render the previous editing mode against the new profile's mode list, acceptable; the `mode_tabs` component is robust to a non-existent editing mode (renders no underline; the next frame corrects it).

### `MetaSnapshot` extensions (`context.rs`)

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MetaSnapshot {
    pub engine_status: EngineStatus,            // existing
    pub current_mode:  String,                  // existing
    pub profile_name:  Option<String>,          // existing
    pub profile_path:  Option<PathBuf>,         // existing
    pub warnings:      Vec<String>,             // existing
    pub mode_force:    Option<ForcedMode>,      // NEW, banner state + runtime-marker amber color
    pub modes:         Vec<String>,             // NEW, flat list from ModeTree::all_modes()
    pub startup_mode:  Option<String>,          // NEW, profile.settings().startup_mode
}

impl MetaSnapshot {
    pub(crate) fn from_state(s: &AppState) -> Self {
        Self {
            engine_status: s.engine_status,
            current_mode:  s.current_mode.clone(),
            profile_name:  s.active_profile.as_ref().map(|p| p.name().to_owned()),
            profile_path:  s.profile_path.clone(),
            warnings:      s.warnings.clone(),
            mode_force:    s.mode_force.clone(),
            modes:         s.active_profile
                            .as_ref()
                            .map(|p| p.modes().all_modes().into_iter().map(str::to_owned).collect())
                            .unwrap_or_default(),
            startup_mode:  s.active_profile.as_ref()
                            .map(|p| p.settings().startup_mode().to_owned()),
        }
    }
}
```

`PartialEq` derived on the whole struct keeps F1's polling-task `Signal::set` no-op-on-equal contract intact, the polling task sets the Signal on every tick, but the gate suppresses re-renders when nothing changed.

**Import path for `ForcedMode`.** `ForcedMode` is reachable via `inputforge_core::state::ForcedMode` without any `pub use` change, `state` is `pub mod` and `ForcedMode` is `pub struct`. The `context.rs` import becomes `use inputforge_core::state::{AppState, DeviceState, EngineStatus, ForcedMode};`.

**Memory-allocation note for steady-state ticks.** `from_state` clones up to 7 fields per call. The polling task gates `Signal::set` on `PartialEq`, so steady-state ticks (no engine state change) pay the projection-clone cost but suppress every downstream allocation. Worth flagging for a future `simplify` pass if profiling reveals the projection clones themselves as a hotspot; out of scope for F7.

**Hierarchy queries do not pass through `MetaSnapshot.modes`.** The `modes` field is intentionally a flat `Vec<String>` (DFS pre-order). Components that need parent/child relationships, currently only the F4 destructive-delete-confirm dialog computing `(modes_count, mappings_count)`, read directly from `ctx.state.read().active_profile.modes()`. Rationale: keeping the snapshot cheap-to-clone and `PartialEq`-stable matters more than centralizing every read, and the consumer surface for hierarchy is small (one rare dialog) and may grow only in F11 (modes panel), which is similarly a tree-view rather than a steady-state subscription.

### `app_root` (`app.rs`) shape

```rust
pub(crate) fn app_root() -> Element {
    let raw    = use_context::<RawHandles>();
    let params = use_context::<LaunchParams>();

    let meta   = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live   = use_signal(LiveSnapshot::default);

    let ctx = AppContext { /* same as F1 */ };
    use_context_provider(|| ctx.clone());

    let view = frame::use_view_state_provider(meta);     // NEW
    use_context_provider(|| view);                        // NEW

    // F4 toast queue, unchanged
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(install_warnings_bridge(ctx.clone(), toast_queue, last_seen));

    // F1 polling, unchanged shape, projection extended
    use_hook(|| spawn_polling_task(ctx.clone()));

    // F3 tray bridge, unchanged
    let tx = use_hook(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<TrayAction>(tray::CHANNEL_CAPACITY);
        tray::spawn_listener_task(rx, ctx.clone());
        tx
    });
    tray::install_event_handler(params.tray_menu_ids.clone(), tx);  // unchanged from F3

    use_hook(|| lifecycle::apply_start_minimized(params.start_minimized));

    rsx! {
        ThemeProvider {
            ToastViewport {}
            frame::Layout {}                              // NEW, replaces PlaceholderShell
        }
    }
}
```

---

## Per-region designs

### Layout (`frame/layout/`)

`Layout` is the single component `app_root` mounts. It owns the layout shell + the no-profile branch.

```rust
#[component]
pub(crate) fn Layout() -> Element {
    let ctx = use_context::<AppContext>();
    let _   = use_context::<ViewState>();   // panics if app_root forgot the provider; do not remove

    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());

    rsx! {
        Stylesheet { href: LAYOUT_CSS }
        TopBar {}
        Banner {}                              // self-renders Hidden when no banner state applies
        if *has_profile.read() {
            div { class: "if-layout__main",
                div { class: "if-layout__rail",   "Mapping list, F8 owns content" }
                div { class: "if-layout__center", "Mapping editor, F9 owns content" }
                PanelSlot {}                   // self-renders empty when slot == None
            }
        } else {
            EmptyState {}                       // F7 stub; F13 replaces
        }
        StatusBar {}
    }
}
```

CSS layout shell, flex column on the outer chrome, flex row on the inner main. (Project rule: every `display` defaults to flex; grid only for true 2D alignment. F7 chrome is two stacked 1D flows, so flex is the canonical fit.)

```css
.if-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
}
.if-top-bar      { flex: 0 0 40px; }
.if-banner       { flex: 0 0 auto; }   /* content height; 0 when component renders rsx!{} */
.if-layout__main { flex: 1 1 auto; min-height: 0; }
.if-status-bar   { flex: 0 0 28px; }

.if-layout__main {
    display: flex;
    flex-direction: row;
}
.if-layout__rail   { flex: 0 0 280px; }
.if-layout__center { flex: 1 1 auto; min-width: 0; }   /* allow shrink past min-content */
.if-panel-slot     { flex: 0 0 320px; }                /* sized when mounted; absent when None */
```

Banner uses `flex: 0 0 auto` so it is content-sized, when the `Banner` component renders an empty fragment in `Hidden` state, no flex item content exists and the row genuinely takes 0px without layout reservation. The panel slot is structurally absent when `panel_slot == None`, so `__center` reclaims the space without an `auto` column track to drain.

### Empty state (`frame/layout/empty_state.rs`)

F7 ships a placeholder; F13 replaces with the real F5-spec workspace empty state. F7 stub renders Display-typography "No profile loaded", enough to validate the no-profile branch.

```rust
#[component]
pub(crate) fn EmptyState() -> Element {
    rsx! {
        Stylesheet { href: EMPTY_STATE_CSS }
        div { class: "if-empty-state",
            div { class: "if-empty-state__heading", "No profile loaded" }
            div { class: "if-empty-state__hint", "F13 owns this surface." }
        }
    }
}
```

### Top bar (`frame/top_bar/`)

`top_bar/mod.rs` is render-only composition:

```rust
#[component]
pub(crate) fn TopBar() -> Element {
    rsx! {
        Stylesheet { href: TOP_BAR_CSS }
        div { class: "if-top-bar",
            EnginePill {}
            div { class: "if-top-bar__divider" }
            ProfileName {}
            ModeTabs {}
            div { class: "if-top-bar__spacer" }   // pushes ToolsCluster right
            ToolsCluster {}
        }
    }
}
```

#### Engine pill (`top_bar/engine_pill/`)

`mod.rs`, render. Subscribes to `meta.engine_status` and `meta.profile_name.is_some()` (no-profile disables the click target).

```rust
#[component]
pub(crate) fn EnginePill() -> Element {
    let ctx = use_context::<AppContext>();
    let status      = use_memo(move || ctx.meta.read().engine_status);
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let commands    = ctx.commands.clone();

    let s = *status.read();
    let p = *has_profile.read();
    let (variant, label, command) = engine_pill_state(s, p);

    rsx! {
        button {
            class: format!("if-engine-pill if-engine-pill--{variant}"),
            disabled: !p,
            "aria-live": "polite",
            role: "status",
            onclick: move |_| { let _ = commands.send(command.clone()); },
            span { class: "if-engine-pill__dot" }
            span { class: "if-engine-pill__label", "{label}" }
        }
    }
}
```

`logic.rs`, pure:

```rust
pub(crate) fn engine_pill_state(
    status: EngineStatus,
    has_profile: bool,
) -> (Variant, &'static str, EngineCommand) {
    match status {
        EngineStatus::Running => (Variant::Live,    "Running", EngineCommand::Deactivate),
        EngineStatus::Paused  => (Variant::Warning, "Paused",  EngineCommand::Activate),
        EngineStatus::Stopped => (Variant::Error,   "Stopped", EngineCommand::Activate),
    }
    // The `disabled: !has_profile` on the button prevents click in the no-profile case;
    // the returned command is unreachable in that case.
}
```

ARIA: `role="status"` + `aria-live="polite"` so transitions are announced; the wrapping element is a real `<button>` for keyboard reachability. Per F5 + F3's existing pattern.

#### Profile name (`top_bar/profile_name.rs`)

Trivial; one file:

```rust
#[component]
pub(crate) fn ProfileName() -> Element {
    let ctx  = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let name = use_memo(move || ctx.meta.read().profile_name.clone());

    let n = name.read().clone();
    rsx! {
        match n {
            Some(s) => rsx! {
                button {
                    class: "if-profile-name if-profile-name--loaded",
                    onclick: move |_| view.panel_slot.set(PanelSlot::Profiles),
                    "{s}"
                }
            },
            None => rsx! {
                span {
                    class: "if-profile-name if-profile-name--empty",
                    "no profile loaded"
                }
            },
        }
    }
}
```

#### Mode tabs (`top_bar/mode_tabs/`)

Heaviest sub-component. `mod.rs` renders the tablist; `logic.rs` is pure derivation; `context_menu.rs` is the right-click/Shift+F10 menu; `add_inline.rs` and `rename_inline.rs` are inline editors that share validation through `logic.rs`.

`logic.rs` API:

```rust
/// Result of computing where the runtime marker dot sits.
#[derive(Debug, PartialEq)]
pub(crate) struct RuntimeMarker {
    pub tab_index: Option<usize>,            // None when no profile / no current mode tab
    pub color:     MarkerColor,              // Natural (green) | Forced (amber)
}
#[derive(Debug, PartialEq)] pub(crate) enum MarkerColor { Natural, Forced }

pub(crate) fn runtime_marker(
    modes: &[String],
    current_mode: &str,
    mode_force: Option<&ForcedMode>,
) -> RuntimeMarker { … }

/// Validation outcome for inline name input. Used by both add_inline and rename_inline.
#[derive(Debug, PartialEq)]
pub(crate) enum NameValidation {
    Valid(String),                            // trimmed
    Empty,
    Duplicate { name: String },
}

pub(crate) fn validate_mode_name(
    raw: &str,
    existing: &[String],
    self_name: Option<&str>,                  // Some(old_name) for rename, exempt self
) -> NameValidation { … }
```

`mod.rs`, render. Uses F2's `Tabs` primitive as the visual base (already implements ARIA tablist + arrow-key focus-roving), but the F7 mode tabs need extras the F3 primitive doesn't carry: per-tab runtime marker dot, per-tab right-click context menu, per-tab inline rename swap, the `+` tail tab, and inline add expansion.

The first concrete planning task is auditing `components/tabs.rs` against these needs. Two outcomes:

- **Preferred:** extend F2's `Tabs` with a per-tab decoration / render-prop API, keeping the ARIA + keyboard contract single-sourced.
- **Fallback:** rebuild the tablist locally in `mode_tabs/mod.rs`, owning ARIA + arrow-key handling directly.

The audit decides. Either way, F7's mode tab renders:

```
[ Default ] [ Combat● ] [ Landing | underline ] [ + ]
              ↑ runtime marker dot, color per mode_force
```

Right-click + Shift+F10 open `context_menu.rs` (F2 `Menu` primitive consumer):

| Item | Action | Disabled when |
|---|---|---|
| Activate | dispatch `EngineCommand::ForceMode { mode: tab.name }` | `mode_force.as_ref().is_some_and(|f| f.mode == tab.name)` |
| Rename | swap tab to inline rename input | profile not loaded |
| Delete | F4 destructive dialog enumerates `(modes_count, mappings_count)` for the subtree (read once at dialog open by walking `ctx.state.read().active_profile`) → on confirm, dispatch `EngineCommand::DeleteMode` | subtree contains `startup_mode`, or tab is the root mode |
| Set as default | dispatch `EngineCommand::SetDefaultMode { name: tab.name }` | tab is already `startup_mode` |

`add_inline.rs`, clicking `+` toggles a local `adding: Signal<bool>`. While true, the `+` is replaced by an F2 `TextInput` with `placeholder: "New mode name"`. The input autofocuses on mount (via `onmounted` + `set_focus(true)`). Submit (Enter) calls `validate_mode_name`:
- **Valid** → dispatch `AddMode { name, parent: None }`, optimistically set `editing_mode = name`, clear local state, close the editor. A parent-side `use_effect` watches `meta.modes` and focuses the new tab once it appears in the snapshot.
- **Empty / Duplicate** → set `error_msg`, render an inline `<span role="alert" aria-live="assertive" id="mode-name-error-add">` under the input, set `aria-invalid="true"` on the input via `aria-describedby="mode-name-error-add"`, **leave focus in the input**.

**Esc** → revert local state, close the editor, restore focus to the `+` button. **Blur** (`onfocusout`) with valid value → commit (Enter path); with empty/invalid → revert (Esc path).

`rename_inline.rs`, same shape. The parent owns `renaming: Signal<Option<String>>`, set to `Some(name)` when the user picks Rename from the context menu. The component reads `from` as a prop and `state` as the parent's signal, closes by setting `state.set(None)`. Validation is `validate_mode_name(raw, modes, Some(from))`, exempting self from duplicate check. Commit dispatches `RenameMode { from, to }` (no-op when `to == from`). On open, the input autofocuses **and selects all** so typing replaces. **Esc** → revert and close, restoring focus to the originating tab. **Blur** with valid → commit; with invalid → revert.

#### Tools cluster (`top_bar/tools_cluster/`)

`mod.rs`:

```rust
#[component]
pub(crate) fn ToolsCluster() -> Element {
    let ctx  = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let slot        = use_memo(move || *view.panel_slot.read());
    let via_calib   = use_memo(move || *view.via_calibration.read());
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());

    let s = *slot.read();
    let v = *via_calib.read();
    let p = *has_profile.read();

    rsx! {
        nav { class: "if-tools-cluster", "aria-label": "Side panels",
            ToolButton {
                active: tool_active(s, v, Tool::Devices),     disabled: !p, label: "Devices",
                onclick: move |_| { view.panel_slot.set(PanelSlot::Devices);  view.via_calibration.set(false); }
            }
            ToolButton {
                active: tool_active(s, v, Tool::Calibration), disabled: !p, label: "Calibration",
                onclick: move |_| { view.panel_slot.set(PanelSlot::Devices);  view.via_calibration.set(true); }
            }
            ToolButton {
                active: tool_active(s, v, Tool::Profiles),    disabled: false, label: "Profiles",
                onclick: move |_| { view.panel_slot.set(PanelSlot::Profiles); view.via_calibration.set(false); }
            }
        }
    }
}
```

`logic.rs`:

```rust
pub(crate) enum Tool { Devices, Calibration, Profiles }

pub(crate) fn tool_active(slot: PanelSlot, via_calibration: bool, tool: Tool) -> bool {
    match (slot, via_calibration, tool) {
        (PanelSlot::Devices,  false, Tool::Devices)     => true,
        (PanelSlot::Devices,  true,  Tool::Calibration) => true,
        (PanelSlot::Profiles, _,     Tool::Profiles)    => true,
        _ => false,
    }
}
```

ARIA: `aria-label="Side panels"` on the wrapping `<nav>`. Each button uses `aria-pressed` (toggle-button semantics) to reflect its active state. Disabled when no profile is loaded (Profiles always enabled, it's the path back to a loaded state).

### Banner (`frame/banner/`)

`logic.rs`:

```rust
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum BannerState {
    Hidden,
    Diverged          { editing: String, current: String },
    /// Forced and aligned: editing == current == forced. The override is active
    /// and the user is editing the mode the engine is currently running.
    Forced            { forced: String },
    ForcedAndDiverged { editing: String, forced: String },
}

pub(crate) fn derive_banner_state(
    editing: &str,
    current: &str,
    mode_force: Option<&ForcedMode>,
) -> BannerState {
    match mode_force {
        None if editing == current => BannerState::Hidden,
        None => BannerState::Diverged {
            editing: editing.to_owned(),
            current: current.to_owned(),
        },
        Some(f) if f.mode == editing => BannerState::Forced { forced: f.mode.clone() },
        Some(f) => BannerState::ForcedAndDiverged {
            editing: editing.to_owned(),
            forced: f.mode.clone(),
        },
    }
}
```

`mod.rs`:

```rust
#[component]
pub(crate) fn Banner() -> Element {
    let ctx      = use_context::<AppContext>();
    let view     = use_context::<ViewState>();
    let commands = ctx.commands.clone();

    let state = use_memo(move || {
        let m = ctx.meta.read();
        let e = view.editing_mode.read().clone();
        derive_banner_state(&e, &m.current_mode, m.mode_force.as_ref())
    });

    let s = state.read().clone();
    rsx! {
        Stylesheet { href: BANNER_CSS }
        match s {
            BannerState::Hidden => rsx! {},
            BannerState::Diverged { editing, current } => rsx! {
                div { class: "if-banner if-banner--diverged",
                    role: "status", "aria-live": "polite",
                    span { class: "if-banner__copy",
                        "Editing " strong {"{editing}"} ", engine is in " strong {"{current}"} "."
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: { let e = editing.clone(); let cmd = commands.clone();
                            move |_| { let _ = cmd.send(EngineCommand::ForceMode { mode: e.clone() }); }},
                        "Activate {editing}"
                    }
                }
            },
            BannerState::Forced { forced } => rsx! {
                div { class: "if-banner if-banner--forced",
                    role: "status", "aria-live": "polite",
                    span { class: "if-banner__copy",
                        "Engine override: " strong {"{forced}"} ". Mode-change rules paused."
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: { let cmd = commands.clone(); move |_| { let _ = cmd.send(EngineCommand::ReleaseMode); }},
                        "Release"
                    }
                }
            },
            BannerState::ForcedAndDiverged { editing, forced } => rsx! {
                div { class: "if-banner if-banner--forced",
                    role: "status", "aria-live": "polite",
                    span { class: "if-banner__copy",
                        "Editing " strong {"{editing}"} ", engine is in " strong {"{forced}"} " (forced). Mode-change rules paused."
                    }
                    Button { variant: ButtonVariant::Secondary,
                        onclick: { let e = editing.clone(); let cmd = commands.clone();
                            move |_| { let _ = cmd.send(EngineCommand::ForceMode { mode: e.clone() }); }},
                        "Activate {editing}"
                    }
                    Button { variant: ButtonVariant::Ghost,
                        onclick: { let cmd = commands.clone(); move |_| { let _ = cmd.send(EngineCommand::ReleaseMode); }},
                        "Release"
                    }
                }
            },
        }
    }
}
```

CSS color tokens (matching F5):

```css
.if-banner--diverged { background: rgba(154, 120, 214, 0.08); border-bottom: 1px solid rgba(154, 120, 214, 0.4); color: var(--color-control); }
.if-banner--forced   { background: rgba(255, 179, 71,  0.10); border-bottom: 1px solid rgba(255, 179, 71,  0.4); color: var(--color-warning); }
```

### Status bar (`frame/status_bar/`)

`logic.rs`:

```rust
pub(crate) fn device_count_label(devices: &[DeviceState]) -> String {
    let connected = devices.iter().filter(|d| d.connected).count();
    format!("{}/{} devices", connected, devices.len())
}

pub(crate) fn warning_count_label(warnings: usize) -> Option<String> {
    match warnings {
        0 => None,
        1 => Some("1 warning".to_owned()),
        n => Some(format!("{n} warnings")),
    }
}

pub(crate) fn truncate_path(path: &Path, max_chars: usize) -> String {
    // Path-middle ellipsis with filename preservation.
    // Final algorithm finalized via impeccable:clarify during implementation.
    …
}
```

`mod.rs`:

```rust
#[component]
pub(crate) fn StatusBar() -> Element {
    let ctx = use_context::<AppContext>();

    let devices_label = use_memo(move || device_count_label(&ctx.config.read().devices));
    let warning_label = use_memo(move || warning_count_label(ctx.meta.read().warnings.len()));
    let path_label    = use_memo(move || ctx.meta.read().profile_path.as_ref()
                                            .map(|p| truncate_path(p, 64)));

    let d = devices_label.read().clone();
    let w = warning_label.read().clone();
    let p = path_label.read().clone();

    rsx! {
        Stylesheet { href: STATUS_BAR_CSS }
        components::StatusBar {
            class: "if-frame-status-bar".to_owned(),
            start: rsx! {
                if let Some(text) = w.as_ref() {
                    Badge { variant: BadgeVariant::Warning, "{text}" }
                }
            },
            middle: rsx! {
                span { "{d}" }
            },
            end: rsx! {
                match p {
                    Some(s) => rsx! { span { class: "if-frame-status-bar__path", "{s}" } },
                    None    => rsx! { span { class: "if-frame-status-bar__path-empty", "-" } },
                }
            },
        }
    }
}
```

Engine status is gone from the start slot (F5 explicit: engine state lives in the top-bar pill, not duplicated). Mode badge gone from the start slot for the same reason. Status bar text never raises to bright per `DESIGN.md`; consumers lift specific badges via their own component.

### Panel slot (`frame/panel_slot/`)

```rust
#[component]
pub(crate) fn PanelSlot() -> Element {
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        match *slot.read() {
            PanelSlot::None => rsx! {},
            PanelSlot::Devices => rsx! {
                aside { class: "if-panel-slot if-panel-slot--devices",
                    "aria-label": "Devices panel",
                    "Devices panel, F12 owns content"
                }
            },
            PanelSlot::Profiles => rsx! {
                aside { class: "if-panel-slot if-panel-slot--profiles",
                    "aria-label": "Profiles panel",
                    "Profiles panel, F13 owns content"
                }
            },
        }
    }
}
```

CSS: fixed-width column (~320px proposed; F12/F13 may revisit), slides in from the right, doesn't dim or trap focus from the rail/center.

---

## Engine surface additions (`crates/inputforge-core/src/`)

### Four new `EngineCommand` variants (`engine/command.rs`)

```rust
pub enum EngineCommand {
    // … existing variants …

    /// Add a new mode under the profile's existing root, or under `parent` if specified.
    /// Default placement: as a child of the root mode (matches "+ creates a new mode" UX).
    AddMode { name: String, parent: Option<String> },

    /// Rename a mode in the active profile's ModeTree. Updates all mappings'
    /// `mode` field and `ProfileSettings::startup_mode` if it referenced the old name.
    RenameMode { from: String, to: String },

    /// Delete a mode (and its descendants) from the active profile. Recursive
    /// cascade: drops all mappings scoped to any deleted mode. Errors if the
    /// mode is the root, or if the subtree contains the profile's
    /// `startup_mode` (must `SetDefaultMode` to a different mode first).
    DeleteMode { name: String },

    /// Set the profile's `ProfileSettings::startup_mode`. Errors if the named
    /// mode doesn't exist in the ModeTree.
    SetDefaultMode { name: String },
}
```

All four variants carry only `String` / `Option<String>` payloads, preserving the existing `Debug + PartialEq` derive on `EngineCommand` (no new bounds required).

**`MoveMode` is deferred** to a later F-task. F7's GUI surface (a flat tablist with no drag-to-reparent affordance) does not expose subtree-reparenting; adding a fifth variant + `ModeTree::with_moved` helper would be unused weight. F11 (modes panel) is the natural home for `MoveMode` if/when the modes-management tree-view UI lands and needs subtree drag-and-drop.

### `Profile` and `ProfileSettings` API additions

The four new handlers depend on bulk-mutation methods that don't exist on `Profile` / `ProfileSettings` today. Add these alongside the existing `set_name` / `set_mapping` / `set_calibrations` pattern:

```rust
// crates/inputforge-core/src/profile/mod.rs
impl Profile {
    /// Replace the mode tree wholesale. Caller is responsible for ensuring the
    /// new tree is consistent with `settings().startup_mode()` and any mode
    /// names referenced by mappings/actions; engine handlers do this validation
    /// before calling.
    pub fn set_modes(&mut self, modes: ModeTree) { … }

    /// Drop every mapping whose `mode` field equals `mode`. Returns the count
    /// of mappings removed (used by `tracing` events and the F4 destructive
    /// dialog's affected-mappings count).
    pub fn remove_mappings_for_mode(&mut self, mode: &str) -> usize { … }

    /// Cascade-rewrite mode references across the action graph: `Mapping.mode`
    /// and every `ModeChangeStrategy::{SwitchTo, Temporary, Cycle}` mode-name
    /// field. Returns the count of mappings whose action graph was touched.
    /// Caller pre-validates that no resulting `CycleModes` would contain a
    /// duplicate (`CycleModes::validated` enforces uniqueness, the rename
    /// handler is the one place that can break that invariant).
    pub fn rename_mode_refs(&mut self, from: &str, to: &str) -> usize { … }
}
```

```rust
// crates/inputforge-core/src/profile/types.rs
impl ProfileSettings {
    /// Set the startup mode. Caller validates the named mode exists in the
    /// profile's ModeTree.
    ///
    /// This is `ProfileSettings`'s first `&mut self` method, it deliberately
    /// breaks the current "immutable after construction" posture rather than
    /// promoting `startup_mode` to a `pub` field. Field-locality + a single
    /// new method is the smaller leak of the two.
    pub fn set_startup_mode(&mut self, mode: String) { … }
}
```

`set_modes` and `remove_mappings_for_mode` are bulk replacements over `Profile`'s private fields. `rename_mode_refs` walks every `Mapping` once, rewriting `Mapping.mode` and any contained `ModeChangeStrategy` mode-name fields in place.

### `ModeTree` mutators (`mode/mod.rs`)

`ModeTree` doesn't currently have functional mutators. F7 adds four pure-by-clone helpers:

```rust
impl ModeTree {
    /// Return a new tree with `name` added as a child of `parent`.
    /// Errors if `name` exists or `parent` doesn't.
    pub fn with_added_child(&self, parent: &str, name: &str) -> Result<Self> { … }

    /// Return a new tree with `from` renamed to `to` everywhere it appears.
    /// Errors if `to` already exists, or `from` doesn't.
    pub fn with_renamed(&self, from: &str, to: &str) -> Result<Self> { … }

    /// Recursive: returns a new tree with `name` and all descendants removed.
    /// Errors if `name` doesn't exist or is the root mode.
    pub fn with_subtree_removed(&self, name: &str) -> Result<Self> { … }

    /// Helper: returns names of all proper descendants of `name`, DFS pre-order.
    /// Errors if `name` doesn't exist. Used by `DeleteMode` to enumerate the
    /// cascade footprint before mutation.
    pub fn descendants_of(&self, name: &str) -> Result<Vec<String>> { … }
}
```

Pure functions over `ModeNode`, testable in isolation without engine plumbing. **First concrete core-side task in the focused plan**, before any handler code.

### Handler placement and shape

Handlers mirror the existing engine command-loop module that handles `ForceMode`/`ReleaseMode`/`SetMapping`/`SetCalibration`. Each handler:

1. Validates the operation against the active profile.
2. Mutates `Profile::modes` (`ModeTree`) or `ProfileSettings::startup_mode`.
3. Cascades affected mappings (`DeleteMode` only).
4. Persists via the existing profile-save path.
5. Emits structured `tracing` events (matches F6's pattern).

Sample handler (`AddMode`):

```rust
fn handle_add_mode(state: &mut AppState, name: String, parent: Option<String>) -> Result<()> {
    let profile = state.active_profile.as_mut().ok_or(EngineError::NoActiveProfile)?;
    let parent  = parent.unwrap_or_else(|| profile.modes().root().name().to_owned());
    let new_tree = profile.modes().with_added_child(&parent, &name)?;
    profile.set_modes(new_tree);
    if let Some(path) = &state.profile_path {
        profile.save(path)?;
    }
    tracing::info!(mode = %name, parent = %parent, "added mode");
    Ok(())
}
```

### Validation rules (engine-enforced)

| Variant | Engine rejects when |
|---|---|
| `AddMode` | name exists; parent doesn't exist; name is empty |
| `RenameMode` | `to` exists (and is not `from`); `from` doesn't exist; `to` is empty; rename would produce a `CycleModes` duplicate in any mapping's action graph |
| `DeleteMode` | name doesn't exist; name is the root mode; subtree contains `ProfileSettings::startup_mode` |
| `SetDefaultMode` | name doesn't exist |

GUI-side validation in `mode_tabs/logic.rs::validate_mode_name` exists for live UX feedback (red border on duplicate-as-you-type). Engine is the source of truth, if the GUI accepts and the engine rejects, the GUI surfaces a toast.

### Cascade semantics

Both `RenameMode` and `DeleteMode` reach into multiple mode-name storage sites. Each handler runs atomically under the engine's write lock; mid-cascade, the polling task can't see partial state.

#### `RenameMode` cascade

Walks all 9 mode-name storage sites the codebase exposes:

**Profile-side (persisted):**
1. `Mapping.mode == from → to` (every matching mapping).
2. `ProfileSettings::startup_mode == from → to`.
3. `ModeChangeStrategy::SwitchTo { mode }` and `::Temporary { mode }`: rewrite `mode` field where `mode == from`.
4. `ModeChangeStrategy::Cycle { modes: CycleModes }`: rewrite every `modes[i] == from → to`, preserving order.

**Engine-side (runtime):**
5. `AppState::current_mode == from → to`.
6. `AppState::mode_force.as_mut().filter(|f| f.mode == from)`, rewrite `f.mode = to`.
7. `ModeState::current == from → to`.
8. `ModeState::stack`: rewrite every entry `== from → to` in place, preserving order.

**Pre-validation:** before any mutation, simulate steps 3-4 against every mapping's action graph. Reject the rename if applying it would produce a `CycleModes` duplicate (`CycleModes::validated` enforces uniqueness, the rename is the one operation that can break that invariant). Engine surfaces the conflicting mapping ID in the error.

**Persistence + tracing:** persist via `Profile::save`; emit `tracing::info!(from, to, mappings_touched, "renamed mode")`.

#### `DeleteMode` cascade

**Recursive**, deleting a mode removes its entire subtree and every mapping scoped to any deleted mode.

**Validation (rejected before any mutation):**
- Name doesn't exist → reject.
- Name is the root mode → reject ("cannot delete root mode").
- Subtree rooted at `name` contains `ProfileSettings::startup_mode` → reject ("change startup mode before deleting"). Caller (the F7 right-click menu) is expected to dispatch `SetDefaultMode` first or steer the user there.

**Cascade (after validation passes):**
1. `deleted: Vec<String>` ← `ModeTree::descendants_of(name)` plus `name` itself.
2. New `ModeTree` ← `with_subtree_removed(name)`, installs via `Profile::set_modes`.
3. For each `m ∈ deleted`: `Profile::remove_mappings_for_mode(&m)`. Sum the counts for telemetry.
4. If `state.current_mode ∈ deleted`: set `current_mode = settings().startup_mode().to_owned()`.
5. If `state.mode_force.as_ref().is_some_and(|f| deleted.contains(&f.mode))`: clear `mode_force` to `None`.
6. If `engine.mode_state.current() ∈ deleted`: reset to `startup_mode`.
7. `engine.mode_state.stack.retain(|m| !deleted.contains(m))`.

**Persistence + tracing:** persist; emit `tracing::info!(modes_deleted = ?deleted, mappings_dropped, "deleted mode subtree")`.

**Why cascade-recursive?** Children cascade because mode hierarchies are organizational, not semantic, a user deleting a parent has implicitly chosen to drop the subtree's organization. Mappings cascade because they're scoped to a deleted mode and would otherwise be unreachable. The hard rejections (root, startup-mode-in-subtree) prevent the two unrecoverable foot-guns: orphaning the entire profile, or breaking the load-time mode resolution. The F4 destructive dialog enumerates `(modes_count, mappings_count)` so the user sees the full footprint before confirming.

### Engine command surface used by F7 (full list, post-additions)

| Variant | F7 dispatch site |
|---|---|
| `Activate` / `Deactivate` | `EnginePill` click |
| `ForceMode { mode }` | Mode-tab right-click → Activate; Banner Activate button |
| `ReleaseMode` | Banner Release button |
| `AddMode { name, parent }` | `add_inline.rs` Enter / blur-with-valid |
| `RenameMode { from, to }` | `rename_inline.rs` Enter / blur-with-valid |
| `DeleteMode { name }` | Mode-tab right-click → Delete (after F4 destructive confirm) |
| `SetDefaultMode { name }` | Mode-tab right-click → Set as default |

F7 does **not** dispatch `LoadProfile`, `SetCalibration`, `SetMapping`, snapshot ops, or `ReloadSettings`. Those belong to F13/F12/F9/F15.

Dispatch is via `ctx.commands.send(...)` (`std::sync::mpsc::Sender<EngineCommand>` per F1's bridge). F3's tray channel uses tokio `mpsc::Sender::try_send`, these are distinct call sites with different APIs, and F7 components copy the F1/F4 pattern, not the F3 tray pattern.

---

## Polling-task projection extension (`bridge.rs`)

`spawn_polling_task` is structurally unchanged. The projection lives in `context.rs::MetaSnapshot::from_state` (extended above). The produced snapshot now has 8 fields instead of 5; `Signal::set` continues to skip on `PartialEq`.

`ConfigSnapshot` and `LiveSnapshot` are untouched. F7 doesn't read `live`.

---

## Render discipline

The contract committed in brainstorming Q10:

1. **Every component reads through narrow `use_memo` slices.** Concrete map:

   | Component | Memos | Re-renders when |
   |---|---|---|
   | `EnginePill` | `engine_status`, `profile_name.is_some()` | engine status flip OR profile load/unload |
   | `ProfileName` | `profile_name` | profile load/unload/rename |
   | `ModeTabs` | `modes`, `current_mode`, `mode_force`, `startup_mode`, `editing_mode` (from ViewState) | mode CRUD, runtime-mode change, force change, default change, editing-tab change |
   | `ToolsCluster` | `panel_slot`, `via_calibration`, `profile_name.is_some()` | panel-slot transition, profile load/unload |
   | `Banner` | `editing_mode`, `current_mode`, `mode_force` (composed into `BannerState` via memo) | any of the three change |
   | `StatusBar` | `devices` (count derived), `warnings.len()`, `profile_path` | device connect/disconnect, warning push, profile load/unload |
   | `PanelSlot` | `panel_slot` | panel slot transition |
   | `Layout` | `profile_name.is_some()` | profile load/unload |

2. **Pure logic in `logic.rs` files**, every non-trivial derivation is a `pub(crate) fn` with no Signal dependencies. All `logic.rs` functions are unit-testable from `#[cfg(test)] mod tests`. No `dioxus::prelude::*` imports in `logic.rs`.

3. **Runtime-marker computation centralized** at the tablist level. `mode_tabs/mod.rs` calls `runtime_marker(...)` once and passes a per-tab boolean (or marker color) into each tab. Avoids each `Tab` independently subscribing to `current_mode` + `mode_force`.

4. **Banner renders the empty fragment** (`rsx! {}`) for `BannerState::Hidden` rather than wrapping in a wrapper that conditionally hides itself, under the outer flex column, an empty fragment produces no flex content and the slot collapses to 0 naturally, no CSS gymnastics.

5. **Steady-state idle target.** With engine running, no input, no user gesture, no warnings: zero F7 re-renders per polling tick. The `MetaSnapshot::from_state` produces an identical struct each tick → `PartialEq` gate → `Signal::set` no-op → no subscribers fire.

---

## Files

**Created (GUI):**

```
crates/inputforge-gui-dx/src/frame/mod.rs
crates/inputforge-gui-dx/src/frame/view_state.rs
crates/inputforge-gui-dx/src/frame/layout/mod.rs
crates/inputforge-gui-dx/src/frame/layout/empty_state.rs
crates/inputforge-gui-dx/src/frame/top_bar/mod.rs
crates/inputforge-gui-dx/src/frame/top_bar/engine_pill/mod.rs
crates/inputforge-gui-dx/src/frame/top_bar/engine_pill/logic.rs
crates/inputforge-gui-dx/src/frame/top_bar/profile_name.rs
crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs
crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/logic.rs
crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs
crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs
crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/rename_inline.rs
crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs
crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs
crates/inputforge-gui-dx/src/frame/banner/mod.rs
crates/inputforge-gui-dx/src/frame/banner/logic.rs
crates/inputforge-gui-dx/src/frame/status_bar/mod.rs
crates/inputforge-gui-dx/src/frame/status_bar/logic.rs
crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs
crates/inputforge-gui-dx/assets/frame/layout.css
crates/inputforge-gui-dx/assets/frame/empty_state.css
crates/inputforge-gui-dx/assets/frame/top_bar.css
crates/inputforge-gui-dx/assets/frame/banner.css
crates/inputforge-gui-dx/assets/frame/status_bar.css
crates/inputforge-gui-dx/assets/frame/panel_slot.css
```

**Modified (GUI):**

```
crates/inputforge-gui-dx/src/lib.rs                 # mod frame; replaces mod shell;
crates/inputforge-gui-dx/src/app.rs                  # provide ViewState; render frame::Layout
crates/inputforge-gui-dx/src/context.rs              # MetaSnapshot gains 3 fields + extended projection; existing `meta_snapshot_default_is_empty` test extends with the 3 new defaults
```

**Deleted (GUI):**

```
crates/inputforge-gui-dx/src/shell/                  # placeholder.rs + status_bar_view.rs + mod.rs
crates/inputforge-gui-dx/assets/shell/               # placeholder-shell.css
```

**Modified / created (core):**

```
crates/inputforge-core/src/engine/command.rs         # 4 new EngineCommand variants
crates/inputforge-core/src/engine/<handlers module>  # 4 new handlers + cascade logic
crates/inputforge-core/src/mode/mod.rs                # ModeTree::with_added_child / with_renamed / with_subtree_removed / descendants_of
crates/inputforge-core/src/profile/mod.rs             # Profile::set_modes / remove_mappings_for_mode / rename_mode_refs
crates/inputforge-core/src/profile/types.rs           # ProfileSettings::set_startup_mode
```

**Reused unchanged:** F2 components (Button, TextInput, Menu, Badge, Tabs primitive, possibly extended for per-tab decoration), F3 `components::StatusBar` primitive, F4 Dialog primitive (for `DeleteMode` confirm), F1 `bridge.rs` polling task, F1 `AppContext`, F3 tray bridge, F3 lifecycle, F4 `ToastQueue`.

---

## Acceptance criteria

- [ ] `cargo build` (default, egui) and `cargo build --no-default-features --features gui-dioxus` both pass with no new warnings vs. F6 baseline.
- [ ] `cargo run --no-default-features --features gui-dioxus` opens a window rendering the F7 frame. With **no profile loaded**: top bar shows engine pill (disabled), italic-muted "no profile loaded" name slot, no mode tabs, tools cluster (Devices/Calibration disabled, Profiles enabled); the rail+center collapse into the `EmptyState` placeholder; status bar shows `0/0 devices` and `-`. With a **profile loaded**: mode tabs render flat-list per `meta.modes` (root mode named per the profile's `ModeTree::root().name()`, typically "Default"); no banner when `editing_mode == current_mode` and `mode_force.is_none()`.
- [ ] **Engine pill** click dispatches `Activate` (when Stopped/Paused) / `Deactivate` (when Running). Visual state and label match the `engine_pill_state` table. Disabled when no profile loaded. ARIA: `role=status` + `aria-live=polite` + `<button>` semantics; Running ↔ Paused transitions announced by AT.
- [ ] **Profile name** displays the active profile name when loaded; renders italic-muted "no profile loaded" when none. Click while loaded sets `panel_slot = Profiles`.
- [ ] **Mode tabs** render the profile's modes flat-list. Active editing tab gets focus-cyan underline; no tab is underlined when `editing_mode ∉ modes` (the brief first-frame-after-profile-load state). Runtime marker dot sits on the tab whose name equals `current_mode`; color is green when `mode_force.is_none()`, amber when `is_some()`. Arrow-key focus roving works. Shift+F10 opens the context menu equivalently to right-click.
- [ ] **Mode-tab `+` inline add** flow: click `+` → text input replaces the `+`; type a name; Enter dispatches `AddMode`; the new tab becomes active editing mode; Esc cancels. Empty/duplicate names show inline error; blur-with-invalid reverts.
- [ ] **Mode-tab right-click menu** items behave per the table: Activate dispatches `ForceMode`; Rename swaps to inline rename; Delete opens F4 destructive dialog with affected-mappings count, on confirm dispatches `DeleteMode`; Set as default dispatches `SetDefaultMode`. Item disabled-states match.
- [ ] **Tools cluster** Devices / Calibration / Profiles toggle `panel_slot` per Replace discipline. Calibration sets `via_calibration = true`. Active styling matches `tool_active`. Devices + Calibration disabled when no profile loaded; Profiles always enabled.
- [ ] **Banner** matches the state machine: Hidden when aligned + unforced, Diverged with `Activate <editing>`, Forced with `Release`, Forced+Diverged with both. Copy is exactly the strings in this spec. ARIA: `role=status` `aria-live=polite`. Buttons dispatch the mapped engine commands.
- [ ] **Status bar** start slot shows warning-count badge (or nothing when 0); middle shows `connected/total devices`; end shows truncated profile path or `-`. No engine status, no mode badge.
- [ ] **Panel slot** mounts F12 placeholder when `Devices`, F13 placeholder when `Profiles`, nothing when `None`. Slides in from the right; doesn't dim or trap focus.
- [ ] **Layout no-profile branch**: when `meta.profile_name.is_none()`, the rail+center collapse into the F7 stub empty state; the banner remains conditional.
- [ ] **MetaSnapshot extension**: `mode_force`, `modes`, `startup_mode` populated correctly from `AppState` in `from_state` tests. `PartialEq` gate suppresses re-renders on identical ticks.
- [ ] **Engine command additions**, `cargo test -p inputforge-core` passes with new tests covering:
  - **AddMode:** happy + duplicate-name + bad-parent + persistence.
  - **RenameMode:** happy + collision + missing-from + reject-on-cycle-duplicate + cascade-update of mappings (`Mapping.mode`) + cascade-update of `ProfileSettings::startup_mode` + cascade-update of action `ModeChangeStrategy` (SwitchTo, Temporary, Cycle) + cascade-update of runtime state (`AppState::current_mode`, `AppState::mode_force`, `ModeState::current`, `ModeState::stack`).
  - **DeleteMode:** happy leaf + happy subtree (recursive) + reject-on-root + reject-when-subtree-contains-startup-mode + cascade reset of `current_mode` + cascade clear of `mode_force` + cascade clear of `ModeState::{current, stack}` references.
  - **SetDefaultMode:** happy + missing-name.
- [ ] **`logic.rs` unit tests** cover `engine_pill_state`, `runtime_marker` (including the no-underline case where `editing_mode ∉ modes`), `validate_mode_name`, `derive_banner_state`, `device_count_label`, `warning_count_label`, `truncate_path` (algorithm-agnostic invariants: respects `max_chars`, preserves filename, uses U+2026 ellipsis, exact algorithm pinned by `impeccable:clarify`), `tool_active`. All pure, no Dioxus runtime.
- [ ] **Steady-state idle render budget**: with engine Running, no devices producing input, no user gesture: zero F7 re-renders per polling tick (verify via Dioxus DevTools subscribe-counter or instrumentation).
- [ ] **F3 cleanup**: `crates/inputforge-gui-dx/src/shell/` directory deleted. `assets/shell/` directory deleted. `app.rs` no longer references `PlaceholderShell`. F3 `StatusBarView` deleted.

---

## Test strategy

- **Pure-function unit tests in each `logic.rs`** under `#[cfg(test)] mod tests`. Cover every match arm of `engine_pill_state`, every quadrant of `derive_banner_state`, every state of `validate_mode_name` (Valid / Empty / Duplicate, with and without `self_name`), every input shape of `runtime_marker` (no profile, current matches, current doesn't match, current matches and forced).
- **Engine-handler integration tests** under `crates/inputforge-core/src/engine/tests.rs`, same shape as existing `ForceMode` tests; cover happy path, validation errors, and cascade effects.
- **`MetaSnapshot::from_state` tests**, extend existing tests in `context.rs` to cover the three new fields. Round-trip a synthetic `AppState` and assert each new field projects correctly.
- **Manual interaction pass** under `cargo run --no-default-features --features gui-dioxus`:
  - Engine pill state transitions.
  - Mode-tab: arrow-key navigation, `+` inline add (happy + duplicate + empty), right-click menu (each item), Shift+F10, inline rename.
  - Banner: load a profile, click a non-active mode tab, verify Diverged banner; click Activate; verify Forced banner; click Release; click another mode while forced; verify Forced+Diverged.
  - Tools cluster: open Devices, open Profiles (replaces Devices), open Calibration (Devices opens with via_calibration = true).
  - Status bar: warning count updates as engine pushes warnings; device count updates on connect/disconnect.
  - Layout: launch with no profile loaded → empty state branch renders; load profile → main row renders.
- **Rendering automation deferred** per parent-plan open question.

---

## Risks

- **F2 `Tabs` primitive may need extension for per-tab decoration.** F7's mode tabs need a runtime-marker dot per tab, plus inline-rename swap, plus a `+` tail tab. If F2's `Tabs` doesn't accept a per-tab render-prop or decoration slot, F7 either extends F2's `Tabs` (preferred, keeps ARIA + keyboard logic single-sourced) or rebuilds the tablist locally (fallback). Mitigation: planning's first concrete code task is auditing `components/tabs.rs` against F7's per-tab needs and deciding extend-vs-rebuild before any `frame/` code is written.
- **Mode-CRUD validation duplication** (engine + GUI). Engine is the source of truth; GUI's `validate_mode_name` exists for live UX feedback. Intentional duplication; logic is short. Mitigation: GUI's validation is a strict subset of engine validation, when in doubt, GUI accepts and engine rejects, surfaced as a toast.
- **Cascade delete is data-destructive.** Cascade is by design but destructive operations on a multi-mapping mode could surprise users mid-tuning. Mitigation: F4 dialog enumerates the affected count; F6 snapshots are the cross-session safety net; per-session undo (F9 territory) covers the in-session case.
- **`AppState` mutation safety under cascade**, engine handlers acquire the write lock, mutate `Profile::modes`, then drop affected mappings. Mid-cascade, the polling task can't see partial state because it holds read lock for the duration of its read. Mitigation: cascade happens inside a single `handle_command` call holding the write lock; tests verify atomicity.
- **Banner render-cost during engine-paused diverged state.** A user with engine paused + editing-tab diverged keeps the banner visible. Each `current_mode` flip from a tray-triggered ForceMode would re-derive `BannerState`. Cost is small (string compare + clone), well below per-tick budget. Flagged for completeness; no mitigation needed.
- **`mode_force` projection is a `clone()`** of the engine's `Option<ForcedMode>`. ForcedMode contains a `String` so this is one allocation per polling tick. With `PartialEq` gate suppressing identical ticks, steady-state cost is one allocation per actual mode_force change. Acceptable.
- **F12/F13 stubs in panel_slot are placeholder text.** F12 and F13 will replace with their real components later. Until then, opening a panel via F7 shows a labeled placeholder, not functional. Documented in acceptance.
- **`view_state.rs` editing-mode reset on profile load.** A `use_effect` watching `meta.profile_name` resets `editing_mode` to the new profile's `startup_mode`. Effect ordering: Dioxus effects run after render, so the next render after a profile load sees the reset value. The first frame after a profile load may briefly render the previous editing mode against the new profile's mode list, `mode_tabs` is robust to a non-existent editing mode (renders no underline; corrects next frame).

---

## Impeccable commands (recommended invocations)

Per F5's pattern; implementation may skip ones that don't apply.

- `impeccable:shape`, at planning time: layout rhythm (top-bar / banner / rail / center / right slot / status-bar), top-bar internal spacing, banner geometry. Single invocation covers the frame as a coherent unit.
- `impeccable:frontend-design`, primary visual treatment of the frame. Engine pill aesthetic, mode-tab focus underline weight, runtime marker dot size and glow, banner backplate treatment, tools-cluster active-state styling. This is the surface the user sees on every launch.
- `impeccable:layout`, top-bar rhythm (engine pill height vs mode tab cap height, gap between zones, divider treatment), banner placement, status-bar slot spacing.
- `impeccable:typeset`, top-bar typography hierarchy, mono-vs-sans for engine status label, tabular figures for device-count and warning-count.
- `impeccable:clarify`, banner copy variants, Activate/Release labels, engine pill hover hint, mode-tab right-click menu items, tools-cluster button labels, profile-path truncation strategy. Tone: terse, functional, no marketing register.
- `impeccable:animate`, engine-pill state transitions (Running ⇌ Paused ⇌ Stopped color shift), banner enter/exit, runtime-marker dot color transition (Natural ⇌ Forced, green ⇌ amber). Honor `prefers-reduced-motion`. No bounce, no overshoot, DESIGN.md cockpit-brisk timing.
- `impeccable:harden`, engine command channel disconnected, profile load failure mid-CRUD-dispatch, mode-delete root/startup-mode rejection toast, RenameMode CycleModes-collision rejection toast, malformed profile, Shift+F10 firing while inline rename is open, focus loss during inline edit when window goes background.
- `impeccable:audit`, keyboard reachability for every interactive element, tab order coherent left-to-right top-to-bottom, focus rings visible against dark backplate, ARIA contracts (engine pill `role=status`+button, banner `role=status`, tools `aria-pressed`, mode tabs `role=tab`/`role=tablist`, panel slot `aria-label`). Color-blind safety on runtime-marker (green ↔ amber pair, verify pattern/glow distinguishability).
- `impeccable:polish`, final pass.

`impeccable:bolder` is **not** invoked for F7, frame stays restrained per `DESIGN.md` ("Most surfaces in the GUI are restrained, the curve editor is permitted to push past safe defaults."). F10/F11 own bold treatment; F7 is the calm surround.

---

## Open questions (deferred)

- **F2 `Tabs` per-tab decoration API shape.** Resolved during F7 implementation kick-off; if extension is invasive, the F2 brainstorm gets a follow-up instead.
- **Banner enter/exit motion.** Slide vs fade vs instant. `impeccable:animate` decides.
- **Mode tab `+` placement** (always-trailing vs scroll-into-view when many modes exist). F5's wireframe shows trailing; F7 ships trailing; revisit only if a profile with many modes overflows the cluster.
- **Profile-path truncation strategy.** `truncate_path(path, 64)` is sketched; exact algorithm is `impeccable:clarify` territory.
- **Shift+F10 on the `+` tab.** F7 default: `+` doesn't open a context menu (it has no operations).
- **F13 reuse of `frame::layout::empty_state`.** F7 ships the placeholder; F13 should replace the contents in place rather than relocating the module. If F13 needs the module path elsewhere, F7's "files deleted" list grows accordingly.
- **Sticky editing-mode persistence in `Preferences`.** Promoted to F15 if anyone asks.
- **Push-based engine→GUI updates.** Post-F17 dedicated feature. Cost analysis: ~2-3 days work using a snapshot-on-notify pattern (engine adds a tiny `state_change_notify: mpsc::Sender<()>` and wakes the GUI listener on every meta/config-touching handler; GUI listener drains and projects from `AppState` as today; live values stay polled at 60Hz). No breaking changes to F7's render contract; coherency preserved by the snapshot-under-read-lock pattern.
- **Testing story for the Dioxus GUI.** Inherited from parent plan.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce a step-by-step implementation plan with TDD-friendly checkpoints. The plan should sequence:
   - **Engine-side first:** `ModeTree::with_added_child` / `with_renamed` / `with_subtree_removed` / `descendants_of` + tests (pure functions, isolated). Then `Profile::set_modes` / `remove_mappings_for_mode` / `rename_mode_refs` and `ProfileSettings::set_startup_mode`. Then `EngineCommand` variants + handlers + cascade tests.
   - **GUI-side audits:** F2 `Tabs` decoration audit (extend vs rebuild decision). `MetaSnapshot` extension + projection.
   - **GUI-side scaffolding:** `ViewState` + provider; `frame::Layout` skeleton + `EmptyState` stub; `app_root` modification.
   - **Per-region build, in dependency order** (simplest signal-bound first):
     - `frame::status_bar` (replaces F3 `StatusBarView`).
     - `frame::engine_pill`, `frame::profile_name`, `frame::tools_cluster`.
     - `frame::banner`.
     - `frame::mode_tabs` last as the heaviest (depends on F2 Tabs audit outcome).
     - `frame::panel_slot` last (just stubs F12/F13).
   - **F3 cleanup:** delete `shell/` + `assets/shell/`.
   - **`impeccable:shape`** invocation early (after the F2 Tabs audit, before `frame/` render code).
   - **`impeccable:frontend-design`** after the structural skeleton stands up; iterate per-region.
   - **Manual interaction pass** against the F5-spec acceptance flows (Authoring, Tuning, Recovery, Discovery, to the extent F7 alone exercises them).
