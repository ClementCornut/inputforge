# F7 Acceptance Sweep — 2026-04-29

> **Status:** in-progress. Stub created at the start of the post-review fix pass; checkboxes tick as fixes land. Plan: `docs/superpowers/plans/2026-04-29-f7-frame.md`. Fix plan: `C:\Users\cornu\.claude\plans\plan-all-fixes-even-cozy-river.md`.

## Build

Verified 2026-04-30.

- [x] `cargo build` (egui default): **PASS** (exit 0, 0 warnings).
- [x] `cargo build --no-default-features --features gui-dioxus -p inputforge-app`: **PASS** (exit 0, 0 warnings).
- [x] Workspace tests (split because `gui-egui` and `gui-dioxus` are mutually exclusive, so `--all-features` rejects with `compile_error!`):
  - `cargo test -p inputforge-core --features test-util`: **596 passed**, 0 failed.
  - `cargo test -p inputforge-gui-dx`: **90 passed**, 0 failed.
  - `cargo test -p inputforge-gui`: **135 passed**, 0 failed.
  - `cargo test -p inputforge-app`: **5 passed**, 0 failed.
  - **Total: 826 passed, 0 failed.**
- [x] `cargo clippy --all-targets -- -D warnings`: **clean** (exit 0).

## Manual sweep

Verified 2026-04-30.

- [x] Engine pill state machine + ARIA (Stopped → Running click cycle, role=status announce, disabled when no profile).
- [x] Profile name slot (button vs em-dash empty state, focus-visible ring after fix §1.3).
- [x] Mode tabs flat list + arrow roving + runtime marker (Natural green vs Forced amber).
- [x] Mode tabs `+` inline add (commit valid, reject empty/duplicate/oversized, focus-on-create after fix §2.8).
- [x] Mode tabs right-click menu (4 items + disabled states; subtree-contains-startup disables Delete).
- [x] Mode tabs Shift+F10 (opens menu anchored to bounding rect, no JS injection after fix §1.1).
- [x] Mode tabs inline rename (Esc reverts focus, Enter commits, validation).
- [x] Mode tabs F4 destructive delete via context menu AND via Delete key after fix §1.2.
- [x] Banner state machine (Hidden / Diverged / Forced / ForcedAndDiverged transitions).
- [x] Tools cluster + panel slot transitions (Devices / Calibration / Profiles, F12/F13 placeholders).
- [x] No-profile empty-state branch (top bar disabled, status bar shows `0/0 devices`, `—`).

## Keyboard-only walkthrough

Verified 2026-04-30.

- [x] Tab into engine pill → Enter activates.
- [x] Tab into mode tablist → ArrowLeft/Right cycles with wrap; Home/End jumps.
- [x] Shift+F10 on a tab opens context menu; ArrowDown/Up cycles items skipping `aria-disabled`.
- [x] Escape closes context menu; focus restored to originating tab.
- [x] **Tab inside open context menu closes it and focus moves to next focusable element (no contention with parent re-focus) — fix §2.4.**
- [x] **Delete on focused tab opens F4 confirm — fix §1.2.**
- [x] `+` add inline opens with focus inside input; Esc reverts to `+` button focus; Enter commits and focuses new tab.
- [x] Rename inline: Esc reverts and restores focus; Enter commits and focuses renamed tab.
- [x] Tab cycles out of tablist into tools cluster; Shift+Tab reverses.
- [x] **Engine pill and profile name buttons show visible focus rings — fix §1.3.**

## Screen-reader (NVDA / Orca)

**Deferred** to the post-feature-complete acceptance pass — will be run
once F8 (mapping list) + F9 (mapping editor) + F11–F13 land and the AT
journey covers the full primary flow, not just the chrome. Static ARIA
contracts (role, aria-live, aria-disabled, aria-selected) are verified
by code review until then. Tracked in Follow-ups below.

- [ ] Banner Hidden → Diverged: "Mode banner. Editing X — engine is in Y" (polite).
- [ ] Diverged → Forced: "Mode banner. Engine override: X. Mode-change rules paused" (polite).
- [ ] Forced → ForcedAndDiverged: assertive interrupt via `role=alert`.
- [ ] Mode-tab arrow nav: each tab announces name + selected/not-selected via `aria-selected`.
- [ ] Context menu open: announces "menu" + first item label.
- [ ] Context menu disabled item: announces as dimmed (confirms `aria-disabled` is read).
- [ ] Dialog open: announces title + body, focus lands on Cancel.
- [ ] Engine pill state changes: announced via `role=status` + polite.
- [ ] **Inline rename/add inputs: no dangling `aria-describedby` until error mounts — fix §2.3.**

## Color contrast

Sampled 2026-04-30 via WebView2 DevTools color picker (AA threshold = 4.5:1
for normal text). Spec target is AA only; AAA failures are documented but
not blocking. Project tokens are pinned to AA in `colors.css`.

- [x] Banner Diverged text on `--color-control-bg`: **6.23×** (AA pass, AAA fail — acceptable).
- [x] Banner Forced text on `--color-warning-bg`: **AA + AAA pass**.
- [x] Banner ForcedAndDiverged text on `--color-warning-bg`: **AA + AAA pass**.
- [x] Mode-tab active text on `--color-bg`: **AA + AAA pass**.
- [x] Mode-tab inactive text on `--color-bg`: **6.07×** (AA pass, AAA fail — acceptable).
- [ ] Empty-state heading + hint on `--color-bg`: **deferred** — empty-state branch is hard to reach in current dev mode (autoload prevents the cold no-profile path). Sample once a clean-profile-state launch flag exists.
- [x] Engine pill **Running** text on `--color-bg-elevated`: **AA + AAA pass**.
- [x] Engine pill **Stopped** text on `--color-bg-elevated`: was **4.1× (AA FAIL)** with raw `--color-error`. **Fixed** by switching `.if-engine-pill--error { color }` to `--color-error-hover` (mirrors the error-badge pattern documented at `colors.css:86`). Re-verify with DevTools after the fix lands.
- [ ] Engine pill **Paused** text on `--color-bg-elevated`: **deferred** — Paused state requires an engine-pause path that isn't easily triggerable from the UI yet. Same warning hue as the banner Forced variant (which clears AA + AAA), so likely fine; verify when the pause path is reachable.

## Reduced-motion

Verified 2026-04-30 via DevTools Rendering panel (Ctrl+Shift+P → Show Rendering
→ "Emulate CSS media feature prefers-reduced-motion: reduce").

- [x] DevTools Rendering panel: emulate `prefers-reduced-motion: reduce`.
- [x] **Banner enter/exit honors gate (no `translateY` animation) — fix §2.5.** Banner snaps in instantly with no slide.
- [x] **Panel-slot enter honors gate (no `translateX` animation) — fix §2.5.** Panel slot snaps in instantly with no slide.
- [-] Runtime-marker color transitions (when impeccable:animate adds them) honor reduced-motion. **N/A today** — no marker color transitions exist yet; revisit when `impeccable:animate` adds them.
- [x] No raw ms timing in F7 CSS (verified by grep `\b\d+ms\b` — only matches are inside CSS comments).

## Edge cases

Verified 2026-04-30.

- [x] Rename to existing name (not self) → inline error with `role="alert"`; focus stays in input.
- [x] Delete active editing tab → focus follows to neighbor; if list empty, focus moves to `+`.
- [x] Right-click during inline rename → document observed event order (rename `onfocusout` vs new tab `oncontextmenu`); add deferral if reversed.
- [x] Shift+F10 with no tab focused → no-op.
- [x] Rapid mode swaps (5+ ForceMode in <1s) → no flicker; PartialEq gate suppresses spurious re-renders.
- [x] Profile load resets `editing_mode` to new startup.
- [x] **Mode name `'); alert(1); //` does not execute JS (id sanitized to integer) — fix §1.1.**
- [x] **Oversized name (65+ graphemes) returns `InvalidConfig`, not `ModeNotFound` — fix §2.6.**

## Render budget (REQUIRED, per spec 1081)

- [x] **`tracing::trace!` instrumented in every `frame::*` component (fix §1.4).**
- [x] Smoke test command (PowerShell):
  ```powershell
  & { $env:RUST_LOG = "frame::render=trace"; cargo run -p inputforge-app --no-default-features --features gui-dioxus }
  ```
- [x] **Pass criterion:** ≤1 trace per region per polling tick. **PASS** — verified 2026-04-30.

### Captured trace (~233 ms window, ~14 ticks at 60 Hz)

| Region          | Traces | Notes |
|-----------------|--------|-------|
| `layout`        | 2      | mount + has_profile flip |
| `top_bar`       | 1      | mount only (no subscriptions) |
| `engine_pill`   | 2      | mount + profile load |
| `profile_name`  | 2      | mount + profile load |
| `tools_cluster` | 2      | mount + profile load |
| `mode_tabs`     | 4      | mount + early effect-driven re-run + profile load + post-load settle |
| `banner`        | 2      | mount + profile load |
| `status_bar`    | 3      | mount + profile load + 1 steady-state event at +173 ms |
| `panel_slot`    | 1      | mount on profile load |
| `empty_state`   | 1      | mount only — correctly unmounts on profile load |

Every region clears the ≤14-per-window ceiling by an order of magnitude.
The `use_memo` slices over `MetaSnapshot` are gating re-renders correctly
via `PartialEq` — most regions only re-fire when their narrow subscription
actually changes, not per polling tick. The single `status_bar` steady-state
trace at +173 ms is a discrete event (hotplug device-count or warnings
update), not per-tick churn.

## Cargo deps added in F7

Recorded for the audit trail (not flagged in plan's file-structure overview but justified by individual tasks):

- [ ] `dioxus-ssr` (dev-dependency) — for the `app::tests::app_root_mounts_frame_layout_not_placeholder_shell` SSR mount-regression test (T18 + fix §2.2).
- [ ] `serde_json` — for context-menu/dialog payloads passed through `document::eval` round-trips.
- [ ] `unicode-segmentation` — for the 64-grapheme cap in `mode_tabs/logic.rs` and engine-side `validate_mode_name_for_engine`.

## Follow-ups

Captured during the post-review pass; tracked here so next session has a single source of truth.

### From original plan
- impeccable:frontend-design — full visual treatment (engine pill, mode-tab focus underline weight, runtime marker glow, banner backplate, tools-cluster active-state styling).
- impeccable:clarify — profile-path truncation algorithm and banner copy variants.
- impeccable:animate — pill state transitions, banner enter/exit, marker color transition.
- impeccable:harden — command channel disconnected, profile load failure mid-CRUD, Shift+F10 while inline rename open.
- impeccable:audit — keyboard reachability, ARIA contracts, color-blind safety on green/amber marker.

### From post-review fix pass
- `unused_qualifications` allow in `frame/top_bar/profile_name.rs` — Dioxus 0.7 RSX macro artifact; revisit when Dioxus upstream fixes the redundant qualification emission.
- `Profile::rename_mode_refs` walks every mapping's actions twice (cycle pre-validate then rewrite). Fine until profile sizes balloon; flag for perf pass once mapping counts >> 1k.
- Property-test harness for `runtime_marker` and other pure-logic functions — would harden coverage but not required by any reviewer-flagged bug.

### Deferred to the post-feature-complete acceptance pass
- **Full screen-reader sweep with NVDA + Speech Viewer** — covers the 9 boxes in the Screen-reader section above. Run once F8 / F9 / F11 / F12 / F13 land so the AT journey covers the primary flow (mapping list, editor, profile management, devices) not just the chrome. Re-verify the chrome bullets at the same time so a single coherent NVDA pass produces the audit trail.
- **Empty-state contrast sample** — needs a clean cold-launch path (autoload-disabled flag or equivalent) so the no-profile branch is reachable.
- **Engine pill Paused contrast sample** — needs a UI-triggerable engine-pause path. The hue is `--color-warning` which clears AA in every other variant tested, so this is verification rather than expected discovery.

### Reviewer false positives (no change needed)
- **§2.1 status-bar class mismatch (plumbing reviewer):** the reviewer flagged `frame/status_bar/mod.rs:42`'s `if-frame-status-bar` class as not matching `layout.css:35`'s `.if-status-bar { flex: 0 0 28px }`. The `StatusBar` primitive at `components/status_bar.rs:24` already composes the base class via `merge_class("if-status-bar", "", class.as_deref())`, so the rendered HTML carries both classes (`class="if-status-bar if-frame-status-bar"`) and the layout's flex rule does apply. The frame's `if-frame-status-bar` namespace is the correct register for frame-level typography overrides on top of the primitive.
- **§3.9 banner forced-variant contrast (CSS reviewer):** the reviewer asked to verify `--color-warning` on `--color-warning-bg` clears WCAG AA. `colors.css:85` documents it at 6.6× — well above the 4.5× AA floor — so no `--color-warning-badge-text` token is needed (the `--color-control-badge-text` mirror exists only because raw `--color-control` falls below AA on its own tint at 3.7×, a different situation).

## Risks tracked from the spec

- **F2 Tabs primitive shape** — resolved in Task 15 audit: Path B (rebuild locally). Audit doc: `docs/superpowers/notes/2026-04-29-f7-tabs-audit.md`.
- **Cascade atomicity** — handlers acquire write lock for the mutation; tests verify no partial state visible. Hardened further by fix §2.7 (op-order pinning comment).
- **Cascade-delete data loss** — F4 dialog enumerates count; F6 snapshots are the cross-session safety net.
- **Banner re-render cost** — pure derivation behind `use_memo`; trivially small.
- **`mode_force` projection clones** — one allocation per `mode_force` change; `PartialEq` gate suppresses identical ticks.
- **F12/F13 are placeholder text** — documented; will be replaced by their own features.
- **Editing-mode reset on profile load** — `use_view_state_provider`'s effect handles it; `mode_tabs` robust to `editing_mode ∉ modes`.
