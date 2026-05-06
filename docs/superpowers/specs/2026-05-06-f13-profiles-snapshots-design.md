# F13 Profiles + Snapshots Design Spec

## Context

F13 replaces the placeholder Profiles panel in the Dioxus GUI with a full
profile lifecycle surface. This spec treats earlier master/F5 notes as prior
context, not binding direction. The approved direction is a library-first
right panel with a panel-scoped snapshot drawer.

InputForge's product register is a dense precision tool for sim-input power
users. The design therefore favors compact rows, keyboard reachability, inline
edits, clear file ownership, and restrained cockpit-panel visual language.

## Goals

- Provide one right-side Profiles panel for profile library management.
- Keep profiles and snapshots visibly linked without letting snapshots dominate
  the library.
- Scope snapshots to the active profile only.
- Keep the main editor layout stable when snapshot history opens or closes.
- Route durable profile and snapshot mutations through engine commands.
- Preserve the no-profile workspace model: center explains, Profiles panel acts.

## Non-Goals

- Managing snapshots for inactive profiles.
- Showing mapping counts in profile rows or snapshot metadata.
- Adding profile sort modes beyond the fixed order.
- Copying snapshot history during Duplicate.
- Turning first launch into a wizard.

## Confirmed Design Choices

### Main Structure

F13 is a Profiles right-panel surface. The main panel shows the profile library.
The active profile is pinned at the top and removed from its normal alphabetical
position. All inactive profiles are sorted alphabetically under it. A simple
filter field narrows the visible profile list.

Profile rows show:

- profile name,
- active pill when the row is active,
- mode count,
- last-edited time when available.

Profile rows must not show mapping counts.

Profile actions live behind a compact overlay menu on each row. The menu renders
over the row and panel surface instead of consuming row height. Inactive rows
offer Open, Rename, Duplicate, Reveal, and Delete. The active row offers Rename,
Duplicate, Reveal, and Delete.

### Snapshot Drawer

Snapshots are scoped to the active profile only. Inactive profile rows do not
show or manage snapshots.

The snapshot drawer is anchored at the bottom of the Profiles panel, not the
whole app. Opening or closing it does not change the center workspace or the
overall app layout. The drawer toggle remains visible in both states and shows:

- a chevron indicating expanded/collapsed state,
- `Snapshots · <active profile name>`,
- snapshot count,
- a compact `+` icon button for "Snapshot now".

When open, the drawer shows a dense newest-first snapshot ledger. Pinned
snapshots keep their newest-first position; pinning protects from eviction but
does not reorder rows.

Snapshot rows show:

- kind marker (auto/manual/pinned),
- timestamp or relative time,
- optional label,
- pinned marker when applicable,
- Restore as the primary row action.

Snapshot rows must not show mapping counts.

### New Profile Flow

`+ New profile` opens a Profiles panel sub-mode rather than an inline row. The
sub-mode has:

- a back/cancel affordance,
- a profile name field,
- source choices for Blank, Copy active, Copy selected, and Open/import path,
- a Create action.

The sub-mode stays inside the right panel. It does not take over the center
workspace.

### File And Library Actions

`Open file...` uses the OS picker. After a valid profile path is chosen, the GUI
offers:

- Load once,
- Add to library.

Load once loads the external profile path without copying it. Add to library
copies the external file into the standard profiles directory, then loads the
library copy.

Duplicate creates a new profile from the selected profile's current
configuration only. It does not copy snapshot history.

Rename moves or renames the snapshot folder with the profile so history follows
the profile.

## Components And Interaction Rules

### Profile Library

Profile rename is inline in-row. Enter or blur commits. Esc cancels and restores
the previous name.

The row menu must be keyboard reachable and must render as an overlay. It should
not resize the row or list when opened. Focus returns to the invoking row/menu
button after menu actions complete or cancel.

Profile delete uses the existing F4 destructive confirmation dialog.

### Snapshot Drawer

The drawer toggle is focusable and announces expanded/collapsed state. The
visual cues are the chevron direction, drawer label, count badge, and bottom
anchoring.

Manual snapshot creation is available through:

- `Ctrl+S` anywhere in the GUI,
- the compact `+` icon in the snapshot drawer toggle/header.

The compact `+` icon has accessible label and tooltip copy of "Snapshot now".

Snapshot restore is visually primary but always requires F4 confirmation. After
confirmation, the Profiles panel stays open and the snapshot drawer remains
visible so the user can inspect or restore again.

Snapshot rename is inline in-row. Enter or blur commits. Esc cancels.

Snapshot delete is immediate and shows a short undo toast. Undo is available
only for the toast window. After the toast window expires, deletion is final.

## Architecture And Data Flow

The GUI should treat the engine/core as the authority for durable profile and
snapshot state. F13 should introduce or use engine commands for profile library
mutations rather than calling profile manager filesystem functions directly from
the GUI.

Profile-library commands include:

- create profile,
- load profile,
- load external profile once,
- add external profile to library,
- rename profile,
- duplicate profile,
- delete profile,
- reveal profile.

Snapshot commands remain engine commands:

- create snapshot,
- restore snapshot,
- rename snapshot,
- pin or unpin snapshot,
- delete snapshot,
- undo recent snapshot delete while the toast window is active.

The GUI owns only presentation state:

- profile filter text,
- selected profile row for copy source,
- open row menu identity,
- active New Profile sub-mode state,
- inline rename drafts and validation text,
- snapshot drawer open/closed state,
- snapshot inline rename draft,
- transient toast/dialog state.

The projected Dioxus context should include profile-library rows and active
snapshot rows. Profile rows need name, active flag, path identity, mode count,
last-edited timestamp when available, and enough identity to dispatch commands.
Snapshot rows need id, kind, label, timestamp, pinned flag, and enough state to
render restore/delete/rename/pin affordances.

Data flow:

1. User acts in the Profiles panel.
2. Dioxus event builds an engine command.
3. Engine/core performs profile or snapshot mutation.
4. App state and settings are updated.
5. Dioxus context projection updates.
6. Panel, dialogs, and toasts rerender from projected state.

The OS file picker is the only OS-owned modal in F13. Once it returns a path,
Load once or Add to library is dispatched through the command path.

## Error Handling And Edge Cases

Profile delete uses F4 destructive confirmation.

Snapshot restore uses F4 confirmation every time, even though restore creates an
auto-before-restore safety snapshot.

Snapshot delete uses immediate deletion plus a short undo toast.

Failed profile create, rename, duplicate, import, or load should keep the user in
the current sub-mode or row edit state and show an inline error. The panel must
not collapse or silently reset draft input.

Failed profile delete or snapshot restore/delete should show a toast. If there
is also local inline context for the failure, keep that context visible.

Missing or corrupt snapshot index data should rely on the core snapshot recovery
behavior. GUI copy should stay factual and should not imply data loss unless the
engine reports data loss.

When no profile is loaded:

- engine is forced Stopped,
- Profiles panel auto-opens,
- center workspace shows the no-profile explanation and New/Open actions,
- Devices and Calibration are disabled,
- mapping list is hidden,
- the snapshot drawer is disabled or replaced with a compact "Load a profile to
  view snapshots" bar.

## Accessibility And Keyboard

- Every row menu opens by keyboard and has a visible focus ring.
- Drawer toggle exposes expanded/collapsed state.
- Inline profile and snapshot rename use Enter, blur, and Esc semantics.
- Dialogs return focus to the invoking control.
- `Ctrl+S` opens the manual snapshot label flow.
- The compact `+` icon exposes "Snapshot now" via accessible label and tooltip.
- Color is not the sole state channel for active, pinned, destructive, or
  expanded states.
- Reduced motion must collapse drawer and menu transitions according to the
  existing motion tokens.

## Visual Guidance

Follow PRODUCT.md and DESIGN.md:

- Sharp, calm, technical.
- Dark cockpit panel vocabulary.
- Dense rows over spacious cards.
- Hairline boundaries and luminance layering.
- No mapping counts in this surface.
- No friendly illustration in the no-profile state.
- No gradient text, decorative glow, glassmorphism, side-stripe cards, or nested
  cards.

The row menu should feel like the existing menu component vocabulary: elevated
surface, hairline border, no row reflow. The snapshot drawer should feel like a
panel region, not a global app drawer.

## Testing And Verification

Implementation should be verified in small slices:

1. Projection tests for profile rows and snapshot rows:
   - active profile pinned first,
   - inactive profiles alphabetical,
   - profile rows omit mapping counts,
   - snapshots newest-first,
   - pinned marker does not reorder.
2. SSR/component tests for:
   - profile panel header actions,
   - row overlay menu,
   - filter field,
   - closed and open drawer states,
   - no inactive snapshot controls,
   - no placeholder copy.
3. Interaction/unit tests for:
   - profile inline rename Enter/Esc/blur,
   - snapshot inline rename Enter/Esc/blur,
   - drawer open/close state,
   - New Profile sub-mode back/cancel.
4. Command dispatch tests for:
   - profile create, rename, duplicate, delete, import, reveal, and load,
   - snapshot create, restore, rename, pin/unpin, delete, and undo delete,
   - `Ctrl+S` opening manual snapshot creation.
5. Error-state tests for:
   - failed create/rename/import inline errors,
   - F4 confirmation for profile delete,
   - F4 confirmation for restore,
   - snapshot delete undo toast window.
6. Visual/manual browser pass for:
   - dark theme density,
   - menu overlay positioning,
   - drawer open/closed affordance,
   - focus rings,
   - reduced motion,
   - desktop and narrow-width text fitting.

## Acceptance Criteria

- Profiles panel replaces the F13 placeholder.
- Active profile is pinned first; inactive profiles are alphabetical.
- Profile rows do not show mapping counts.
- Profile row actions render in overlay menus without row reflow.
- New Profile uses a Profiles panel sub-mode.
- Open file supports Load once and Add to library.
- Duplicate does not copy snapshots.
- Rename carries the snapshot folder with the profile.
- Snapshot drawer is scoped to the right panel.
- Snapshot drawer can be opened and closed with clear visual and accessible
  affordances.
- Snapshot rows do not show mapping counts.
- Restore is primary and confirmed.
- Snapshot delete uses a short undo toast.
- Durable profile and snapshot mutations go through engine commands.
- No-profile state keeps center explanation plus actionable Profiles panel.
