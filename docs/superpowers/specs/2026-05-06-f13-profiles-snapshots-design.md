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

The active profile remains visible at the top when a filter is active, even if
its name does not match the filter. Inactive rows filter normally. When no
inactive rows match, the list shows a compact empty state under the pinned
active row. The snapshot drawer remains scoped to the active profile and is not
affected by profile-list filtering.

An external Load once profile appears as the pinned active row with an
`External` badge. It is not inserted into the normal library list. Its active
row offers Duplicate, Reveal, Add to library, and Snapshot now. Rename and
Delete are hidden for external Load once profiles because the external source
file is not owned by InputForge.

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

- kind marker (auto/manual),
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
- source choices for Blank, Copy active, Copy source profile, and Open/import
  path,
- a Create action.

Copy source profile uses a select control inside the sub-mode, populated from
available library profiles. It does not depend on the currently filtered or
highlighted library row, because the library list is not the active view while
the sub-mode is open. The Create action is disabled until the selected
source/path/name is valid for the chosen source type.

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

Load once keeps the external source file outside InputForge ownership. Snapshots
for a loaded-once external profile use an app-managed snapshot namespace keyed
by a stable hash of the external file's canonical path. Reloading the same
external path finds the same external snapshot history. Add to library copies
only the profile file; it does not migrate external snapshot history into the
new library profile's snapshot folder.

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

Deleting an active library profile uses the same F4 confirmation. On success,
core deletes the profile file and its snapshot folder, clears `last_profile`,
clears active profile state, forces the engine Stopped, and transitions the GUI
to the no-profile state. The app does not auto-load another profile after active
profile deletion. If deletion fails, the active profile stays loaded and the GUI
shows a failure toast.

### Snapshot Drawer

The drawer toggle is focusable and announces expanded/collapsed state. The
visual cues are the chevron direction, drawer label, count badge, and bottom
anchoring.

Manual snapshot creation is available through:

- `Ctrl+S` when focus is not inside editable or modal UI,
- the compact `+` icon in the snapshot drawer toggle/header.

The compact `+` icon has accessible label and tooltip copy of "Snapshot now".
When focus is inside a text input, inline rename field, menu, dialog, or
OS-picker return flow, F13 snapshot handling ignores `Ctrl+S`; the focused
control owns the key. The compact `+` button remains available when the shortcut
is suppressed.

Snapshot restore is visually primary but always requires F4 confirmation. After
confirmation, the Profiles panel stays open and the snapshot drawer remains
visible so the user can inspect or restore again.

Snapshot rename is inline in-row. Enter or blur commits. Esc cancels.

Snapshot delete is visually immediate and shows a short undo toast. The engine
owns deletion as a pending-delete operation: the snapshot disappears from the
ledger immediately, but core moves it to a recoverable pending-delete store
instead of letting the GUI keep file payloads. Undo dispatches the engine undo
command during the toast window. Toast expiry finalizes purge through core. If
the app exits before the toast expires, pending deletes are purged on next
startup.

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
- New Profile source selector state,
- open row menu identity,
- active New Profile sub-mode state,
- inline rename drafts and validation text,
- snapshot drawer open/closed state,
- snapshot inline rename draft,
- pending snapshot-delete toast identity,
- transient toast/dialog state.

The projected Dioxus context should include profile-library rows and active
snapshot rows. Profile rows need name, active flag, path identity, mode count,
last-edited timestamp when available, profile origin (`library` or `external`),
external badge state, action availability, snapshot namespace identity, and
enough identity to dispatch commands. External Load once rows also need the
canonical-path-hash identity used for app-managed snapshot storage. Snapshot
rows need id, kind, label, timestamp, pinned flag, pending-delete absence from
the visible ledger, and enough state to render restore/delete/rename/pin
affordances.

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

Snapshot delete removes the row from the visible ledger immediately and shows a
short undo toast.

Snapshot delete must not let GUI code remove snapshot files directly. Core owns
the pending-delete store, undo window, final purge, and startup cleanup of
expired pending deletes.

Failed profile create, rename, duplicate, import, or load should keep the user in
the current sub-mode or row edit state and show an inline error. The panel must
not collapse or silently reset draft input.

Failed profile delete or snapshot restore/delete should show a toast. If there
is also local inline context for the failure, keep that context visible.

Missing or corrupt snapshot index data should rely on the core snapshot recovery
behavior. GUI copy should stay factual and should not imply data loss unless the
engine reports data loss.

The GUI pre-validates common profile-library inputs before dispatch when it has
enough local information:

- empty or whitespace-only profile names,
- filename-illegal characters,
- duplicate library profile names or destination paths,
- missing external paths,
- unsupported or corrupt profile files detected during import/load,
- permission errors reported by the OS or engine,
- snapshot-folder rename collisions,
- case-only profile renames.

Case-only profile rename is allowed when core supports the destination move.
Validation errors in create, rename, duplicate, import, or load flows stay
inline, preserve the draft value, and leave the current sub-mode or row edit
open. Destructive or operation-level failures use toasts, with inline context
preserved when it exists.

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
- `Ctrl+S` opens the manual snapshot label flow only when focus is outside text
  inputs, inline rename fields, menus, dialogs, and OS-picker return flow.
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
   - active row remains visible while filtering,
   - external Load once row projects origin, badge, actions, and snapshot
     namespace identity,
   - snapshots newest-first,
   - pinned marker does not reorder,
   - pending-delete snapshots are absent from the visible ledger.
2. SSR/component tests for:
   - profile panel header actions,
   - row overlay menu,
   - filter field,
   - filtered empty state,
   - closed and open drawer states,
   - no inactive snapshot controls,
   - no placeholder copy.
3. Interaction/unit tests for:
   - profile inline rename Enter/Esc/blur,
   - snapshot inline rename Enter/Esc/blur,
   - drawer open/close state,
   - New Profile sub-mode back/cancel,
   - New Profile source select enable/disable rules,
   - `Ctrl+S` suppressed in editable and modal UI.
4. Command dispatch tests for:
   - profile create, rename, duplicate, delete, import, reveal, and load,
   - active profile delete to no-profile state,
   - Load once external snapshot namespace by canonical path hash,
   - snapshot create, restore, rename, pin/unpin, delete, and undo delete,
   - `Ctrl+S` opening manual snapshot creation.
5. Error-state tests for:
   - failed create/rename/import inline errors,
   - empty names, illegal filename characters, duplicates, missing external
     paths, corrupt profile files, permission errors, snapshot-folder rename
     collisions, and case-only renames,
   - F4 confirmation for profile delete,
   - F4 confirmation for restore,
   - snapshot delete pending-delete undo window and startup purge.
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
- New Profile chooses copy sources through an in-sub-mode select control.
- Open file supports Load once and Add to library.
- Load once shows an external active row with limited safe actions.
- External Load once snapshots use an app-managed canonical-path-hash namespace.
- Add to library does not migrate external snapshot history.
- Duplicate does not copy snapshots.
- Rename carries the snapshot folder with the profile.
- Active library profile delete clears the active profile and enters no-profile
  state without auto-loading another profile.
- Snapshot drawer is scoped to the right panel.
- Snapshot drawer can be opened and closed with clear visual and accessible
  affordances.
- Snapshot rows do not show mapping counts.
- Restore is primary and confirmed.
- Snapshot delete uses an engine-owned pending-delete undo toast and purges
  expired pending deletes on startup.
- `Ctrl+S` opens manual snapshot creation only outside editable and modal UI.
- GUI validation covers empty names, illegal filename characters, duplicate
  library names, missing/corrupt external paths, permission errors,
  snapshot-folder collisions, and case-only rename behavior.
- Durable profile and snapshot mutations go through engine commands.
- No-profile state keeps center explanation plus actionable Profiles panel.
