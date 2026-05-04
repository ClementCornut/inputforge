# Mapping List Device Filter And vJoy Badges Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the mapping list usable with large profiles by removing noisy unnamed labels, adding one-click device filtering, showing clear vJoy output badges, and keeping Add Mapping reachable.

**Architecture:** Extend `ConfigSnapshot` once per polling tick with row-level facts (`referenced_devices`, `first_vjoy_output`) so rendering and filtering stay cheap. Keep pure filtering/label helpers in the mapping-list module, then wire Dioxus state and CSS around those helpers.

**Tech Stack:** Rust 2024, Dioxus 0.7, SSR component tests, CSS assets in `crates/inputforge-gui-dx/assets/frame/mapping_list.css`.

---

## File Structure

- Modify `crates/inputforge-gui-dx/src/context.rs`: add `MappingSummary` fields and deterministic action-tree extraction helpers.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`: add device-filter and device-chip pure helpers beside `matches_filter`.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`: remove `(unnamed)` row title fallback and render `first_vjoy_output` badge.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`: own selected-device state, render chip strip, combine filters, clear stale device selection, and place AddInline in a sticky footer.
- Modify `crates/inputforge-gui-dx/assets/frame/mapping_list.css`: single-row chip strip, output badge, unnamed-row spacing, sticky footer.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`: integration/SSR coverage for rail behavior.

## Task 1: Snapshot Row Facts

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write failing context tests**

Add tests in `context.rs` under the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn mapping_summary_referenced_devices_dedupes_and_ignores_unbound() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, MergeOp};
    use std::collections::HashMap;

    let dev_a = DeviceId("dev-a".to_owned());
    let primary = InputAddress::Bound {
        device: dev_a.clone(),
        input: InputId::Axis { index: 0 },
    };
    let modes = ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![
                Action::MergeAxis {
                    second_input: InputAddress::Unbound,
                    operation: MergeOp::Sum,
                },
                Action::Conditional {
                    condition: Condition::ButtonPressed { input: primary },
                    if_true: vec![],
                    if_false: vec![],
                },
            ],
        }],
        vec![],
        "Default".to_owned(),
    );

    let cfg = ConfigSnapshot::from_state(&AppState::with_profile(profile), None);
    assert_eq!(cfg.mappings[0].referenced_devices, vec![dev_a]);
}

#[test]
fn mapping_summary_finds_first_vjoy_output_in_preorder() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    let modes = ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
    let input = InputAddress::Bound {
        device: DeviceId("stick".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let true_output = OutputAddress {
        device: 2,
        output: OutputId::Axis { id: VJoyAxis::Y },
    };
    let false_output = OutputAddress {
        device: 3,
        output: OutputId::Axis { id: VJoyAxis::Z },
    };
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: input.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed { input },
                if_true: vec![Action::MapToVJoy {
                    output: true_output.clone(),
                }],
                if_false: vec![Action::MapToVJoy {
                    output: false_output,
                }],
            }],
        }],
        vec![],
        "Default".to_owned(),
    );

    let cfg = ConfigSnapshot::from_state(&AppState::with_profile(profile), None);
    assert_eq!(cfg.mappings[0].first_vjoy_output.as_ref(), Some(&true_output));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx mapping_summary_ --lib`

Expected: compile failure because `MappingSummary` has no `referenced_devices` or `first_vjoy_output` fields.

- [ ] **Step 3: Implement snapshot fields and helpers**

In `context.rs`, import `DeviceId` and `OutputAddress` where needed, extend `MappingSummary`, and add helpers:

```rust
pub(crate) struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
    pub referenced_devices: Vec<DeviceId>,
    pub first_vjoy_output: Option<OutputAddress>,
}
```

Implement helpers near `derive_glyphs`:

```rust
fn derive_referenced_devices(
    primary: &InputAddress,
    actions: &[inputforge_core::action::Action],
) -> Vec<DeviceId> {
    fn push_addr(out: &mut Vec<DeviceId>, addr: &InputAddress) {
        if let Some(device) = addr.device() {
            if !out.iter().any(|existing| existing == device) {
                out.push(device.clone());
            }
        }
    }

    fn walk_condition(out: &mut Vec<DeviceId>, condition: &inputforge_core::action::Condition) {
        use inputforge_core::action::Condition;
        match condition {
            Condition::ButtonPressed { input }
            | Condition::ButtonReleased { input }
            | Condition::AxisInRange { input, .. }
            | Condition::HatDirection { input, .. } => push_addr(out, input),
            Condition::All { conditions } | Condition::Any { conditions } => {
                for child in conditions {
                    walk_condition(out, child);
                }
            }
            Condition::Not { condition } => walk_condition(out, condition),
        }
    }

    fn walk_actions(out: &mut Vec<DeviceId>, actions: &[inputforge_core::action::Action]) {
        use inputforge_core::action::Action;
        for action in actions {
            match action {
                Action::MergeAxis { second_input, .. } => push_addr(out, second_input),
                Action::Conditional {
                    condition,
                    if_true,
                    if_false,
                } => {
                    walk_condition(out, condition);
                    walk_actions(out, if_true);
                    walk_actions(out, if_false);
                }
                _ => {}
            }
        }
    }

    let mut out = Vec::new();
    push_addr(&mut out, primary);
    walk_actions(&mut out, actions);
    out
}

fn first_vjoy_output(actions: &[inputforge_core::action::Action]) -> Option<OutputAddress> {
    use inputforge_core::action::Action;
    for action in actions {
        match action {
            Action::MapToVJoy { output } => return Some(output.clone()),
            Action::Conditional {
                if_true, if_false, ..
            } => {
                if let Some(output) = first_vjoy_output(if_true) {
                    return Some(output);
                }
                if let Some(output) = first_vjoy_output(if_false) {
                    return Some(output);
                }
            }
            _ => {}
        }
    }
    None
}
```

Populate the fields in `ConfigSnapshot::from_state`.

Before expecting this task's tests to pass, compile-fix all existing `MappingSummary { ... }` literals affected by the new fields. Known fallout exists in `mapping_list/filter.rs` helper/test rows and in several `mapping_list/tests.rs` component fixtures. Either add `referenced_devices` and `first_vjoy_output` to each literal directly, or introduce a shared `mapping_summary_for_test(...)` helper in `mapping_list/tests.rs` and convert the repeated fixtures to use it.

- [ ] **Step 4: Run context tests**

Run: `cargo test -p inputforge-gui-dx mapping_summary_ --lib`

Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): summarize device refs and vjoy output"
```

## Task 2: Device Filter Pure Helpers

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`

- [ ] **Step 1: Write failing helper tests**

Add tests for `matches_device_filter` and chip derivation. Use `DeviceId("dev-a")`, `DeviceId("dev-b")`, duplicate names, and a text query that should not affect chips.

```rust
#[test]
fn device_filter_matches_referenced_devices() {
    let row = row_with_refs("Axis", vec!["dev-a", "dev-b"]);
    assert!(matches_device_filter(&row, Some(&DeviceId("dev-b".to_owned()))));
    assert!(!matches_device_filter(&row, Some(&DeviceId("dev-c".to_owned()))));
    assert!(matches_device_filter(&row, None));
}

#[test]
fn device_chips_are_current_mode_first_seen_and_disambiguated() {
    let cfg = cfg_with_named_devices([
        ("dev-a", "Twin Stick"),
        ("dev-b", "Twin Stick"),
        ("dev-c", "Pedals"),
    ]);
    let rows = vec![
        row_in_mode_with_refs("Default", "A", vec!["dev-b"]),
        row_in_mode_with_refs("Other", "Other", vec!["dev-c"]),
        row_in_mode_with_refs("Default", "B", vec!["dev-a"]),
        row_in_mode_with_refs("Default", "C", vec!["dev-b"]),
    ];

    let chips = device_chips_for_mode(&rows, "Default", &cfg);
    assert_eq!(chips.iter().map(|c| c.id.0.as_str()).collect::<Vec<_>>(), vec!["dev-b", "dev-a"]);
    assert_ne!(chips[0].label, chips[1].label);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx mapping_list::filter --lib`

Expected: compile failure for missing helpers.

- [ ] **Step 3: Implement helper API**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceChip {
    pub id: DeviceId,
    pub label: String,
}

pub(crate) fn matches_device_filter(row: &MappingSummary, selected: Option<&DeviceId>) -> bool {
    selected.is_none_or(|device| row.referenced_devices.iter().any(|d| d == device))
}

pub(crate) fn device_chips_for_mode(
    rows: &[MappingSummary],
    mode: &str,
    cfg: &ConfigSnapshot,
) -> Vec<DeviceChip> {
    let mut ids: Vec<DeviceId> = Vec::new();
    for row in rows.iter().filter(|row| row.mode == mode) {
        for device in &row.referenced_devices {
            if !ids.iter().any(|existing| existing == device) {
                ids.push(device.clone());
            }
        }
    }

    let mut chips: Vec<DeviceChip> = ids
        .into_iter()
        .map(|id| {
            let label = cfg
                .devices
                .iter()
                .find(|device| device.info.id == id)
                .map(|device| device.info.name.clone())
                .unwrap_or_else(|| id.0.clone());
            DeviceChip { id, label }
        })
        .collect();

    let mut counts = std::collections::HashMap::<String, usize>::new();
    for chip in &chips {
        *counts.entry(chip.label.clone()).or_default() += 1;
    }
    for chip in &mut chips {
        if counts.get(&chip.label).copied().unwrap_or_default() > 1 {
            chip.label = format!("{} · {}", chip.label, chip.id.0);
        }
    }
    chips
}
```

The label fallback order is current connected device name, then raw ID. There is no profile-known name source yet, so do not add a dead cache field in this pass.

- [ ] **Step 4: Run helper tests**

Run: `cargo test -p inputforge-gui-dx mapping_list::filter --lib`

Expected: filter module tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs
git commit -m "feat(mapping-list): add device filter helpers"
```

## Task 3: Row Name And vJoy Badge

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write failing row tests**

Add SSR tests asserting:

```rust
#[test]
fn row_omits_unnamed_placeholder_when_not_renaming() {
    // Mount Row with name: None.
    // Assert rendered HTML does not contain "(unnamed)".
    // Assert source device/input text remains visible.
}

#[test]
fn row_renders_compact_vjoy_output_badge() {
    // Mount Row with first_vjoy_output = Some(OutputAddress { device: 2, output: OutputId::Axis { id: VJoyAxis::X } }).
    // Assert HTML contains "vJoy 2" and "X".
    // Assert HTML contains class "if-row__output-badge".
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx row_ --lib`

Expected: unnamed test fails because `(unnamed)` is still rendered; badge test fails because no badge exists.

- [ ] **Step 3: Implement row changes**

In `row.rs`, add an output label helper:

This helper intentionally duplicates the existing private editor/live-readout output-label convention locally for this pass. Do not add a shared formatter refactor here; centralization can happen later if more GUI surfaces need the same API.

```rust
fn compact_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X",
            VJoyAxis::Y => "Y",
            VJoyAxis::Z => "Z",
            VJoyAxis::Rx => "Rx",
            VJoyAxis::Ry => "Ry",
            VJoyAxis::Rz => "Rz",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Btn {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} · {}", output.device, suffix)
}
```

Render `if let Some(name)` only for named rows. Do not render the `<em>(unnamed)</em>` branch. Render the badge beside the primary source line:

```rust
if let Some(output) = &summary.first_vjoy_output {
    span {
        class: "if-row__output-badge",
        title: "{compact_output_label(output)}",
        "{compact_output_label(output)}"
    }
}
```

- [ ] **Step 4: Run row tests**

Run: `cargo test -p inputforge-gui-dx row_ --lib`

Expected: row tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/row.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): show compact vjoy badges"
```

## Task 4: Rail Device Chips And Sticky Add Mapping

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write failing rail tests**

Add tests:

```rust
#[test]
fn mapping_list_renders_single_row_device_filter_chips() {
    // Seed two Default mappings referencing two connected devices.
    // Render MappingList and assert:
    // - "if-rail__device-filter" is present.
    // - role="group" and aria-label="Filter mappings by device" are present.
    // - each device label appears exactly once as chip text/title.
}

#[test]
fn mapping_list_device_chips_are_toggle_buttons() {
    // Seed one Default mapping referencing one device.
    // Render MappingList and assert the chip has:
    // - class "if-rail__device-chip".
    // - type="button".
    // - aria-pressed="false".
    // - a title equal to the visible device label.
}

#[test]
fn mapping_list_add_inline_is_in_sticky_footer() {
    // Mount MappingList with several mappings.
    // Assert "if-rail__scroll" wraps the mapping groups.
    // Assert AddInline appears under "if-rail__add-sticky".
}

#[test]
fn mapping_list_css_keeps_device_chips_one_row() {
    let css = include_str!("../../../../assets/frame/mapping_list.css");
    assert!(css.contains(".if-rail__device-filter"));
    assert!(css.contains("overflow-x: auto"));
    assert!(css.contains("overflow-y: hidden"));
    assert!(css.contains("flex-wrap: nowrap"));
    assert!(css.contains(".if-rail__device-chip"));
    assert!(css.contains("flex: 0 0 auto"));
    assert!(css.contains("white-space: nowrap"));
    assert!(css.contains(".if-rail__device-chip:focus-visible"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx mapping_list_ --lib`

Expected: tests fail for missing classes/markup.

- [ ] **Step 3: Implement MappingList state and markup**

In `MappingList`, add:

```rust
let selected_device: Signal<Option<DeviceId>> = use_signal(|| None);
```

Compute chips from current-mode rows before text filtering:

```rust
let device_chips_memo = use_memo(move || {
    let cfg = ctx.config.read();
    let mode_now = editing.read().clone();
    device_chips_for_mode(&cfg.mappings, &mode_now, &cfg)
});
```

Filter rows with both predicates:

```rust
if matches_filter(m, &query, &cfg)
    && matches_device_filter(m, selected_device.read().as_ref())
{
    filtered.push(m.clone());
}
```

Render chips above groups:

```rust
DeviceFilterRow {
    chips: device_chips_memo.read().clone(),
    selected: selected_device,
}
```

Add a small component in `mod.rs`:

```rust
#[component]
fn DeviceFilterRow(chips: Vec<DeviceChip>, selected: Signal<Option<DeviceId>>) -> Element {
    if chips.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "if-rail__device-filter", role: "group", "aria-label": "Filter mappings by device",
            for chip in chips {
                {
                    let active = selected.read().as_ref() == Some(&chip.id);
                    let id = chip.id.clone();
                    let label = chip.label.clone();
                    rsx! {
                        button {
                            class: if active { "if-rail__device-chip is-active" } else { "if-rail__device-chip" },
                            r#type: "button",
                            "aria-pressed": if active { "true" } else { "false" },
                            title: "{label}",
                            onclick: move |_| {
                                let mut selected = selected;
                                if selected.peek().as_ref() == Some(&id) {
                                    selected.set(None);
                                } else {
                                    selected.set(Some(id.clone()));
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
```

Wrap groups in a scroll area and AddInline in sticky footer:

```rust
div { class: "if-rail__scroll",
    { group_iter }
}
div { class: "if-rail__add-sticky",
    AddInline { force_expanded: force_expand_add }
}
```

Also clear selected device with an effect when the current mode changes or chip derivation changes and `selected_device` is no longer present in `device_chips_for_mode`. The effect should compare by `DeviceId`, and leave the selection intact when the same `DeviceId` still exists under a different display label.

- [ ] **Step 4: Add CSS**

In `mapping_list.css`, add:

```css
.if-rail {
    min-height: 0;
}

.if-rail__device-filter {
    display: flex;
    flex-wrap: nowrap;
    gap: var(--space-1);
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--space-1) var(--space-3) var(--space-2);
    scrollbar-width: thin;
    flex: 0 0 auto;
}

.if-rail__device-chip {
    flex: 0 0 auto;
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.if-rail__device-chip.is-active {
    border-color: var(--color-primary);
    color: var(--color-primary);
}

.if-rail__device-chip:focus-visible {
    outline: 2px solid var(--color-focus);
    outline-offset: 2px;
}

.if-rail__scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: calc(var(--space-8) + var(--space-3));
}

.if-rail__add-sticky {
    position: sticky;
    bottom: 0;
    flex: 0 0 auto;
    background: var(--color-bg);
    border-top: 1px solid var(--color-border);
    padding: var(--space-2) 0;
}
```

Adjust existing `.if-add-inline` margin if needed so the sticky footer does not double-stack vertical padding.

- [ ] **Step 5: Run rail tests**

Run: `cargo test -p inputforge-gui-dx mapping_list_ --lib`

Expected: mapping-list tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/assets/frame/mapping_list.css crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): add device chips and sticky add"
```

## Task 5: Empty-State Clear Actions

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write failing empty-state tests**

Add a component test for `EmptyZeroFilterResults` with text and device clear handlers. Assert it renders `Clear text` and `Clear device`.

Use this concrete fixture:

```rust
#[test]
fn mapping_list_zero_filter_exposes_clear_actions() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "throttle".to_owned(),
                device_label: Some("Twin Stick".to_owned()),
                on_clear_text: move |()| {},
                on_clear_device: Some(EventHandler::new(move |()| {})),
            }
        }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Clear text"), "text clear action missing: {html}");
    assert!(html.contains("Clear device"), "device clear action missing: {html}");
    assert!(html.contains("Twin Stick"), "device label missing from zero-filter state: {html}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx empty_zero_filter --lib`

Expected: compile failure or assertion failure because only the current clear action exists.

- [ ] **Step 3: Implement clear action API**

Change `EmptyZeroFilterResults` props to accept:

```rust
query: String,
device_label: Option<String>,
on_clear_text: EventHandler<()>,
on_clear_device: Option<EventHandler<()>>,
```

Render the zero-result message so it represents both active filters: include the text query when `query.trim()` is non-empty and include the selected device label when `device_label.is_some()`. Render `Clear text` when `query.trim()` is non-empty and `Clear device` when `device_label.is_some()`. Both actions must use the existing `Button { variant: ButtonVariant::Ghost }` styling inside the empty-state action area, with independent handlers.

Update `MappingList` zero-filter branch to pass both handlers.

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx empty_zero_filter --lib`

Expected: empty-state tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): add clear actions for filters"
```

## Task 6: Final Verification

**Files:**
- No planned source edits.

- [ ] **Step 1: Run focused GUI tests**

Run: `cargo test -p inputforge-gui-dx mapping_list --lib`

Expected: all mapping-list tests pass.

- [ ] **Step 2: Run broader GUI tests**

Run: `cargo test -p inputforge-gui-dx --lib`

Expected: all `inputforge-gui-dx` library tests pass.

- [ ] **Step 3: Run formatting and clippy**

Run:

```bash
cargo fmt --check
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
```

Expected: both commands exit 0.

- [ ] **Step 4: Re-index code**

Run jcodemunch incremental index for `E:\Git\Perso\inputforge` with AI summaries disabled.

Expected: index succeeds and includes changed Rust/CSS files.

- [ ] **Step 5: Commit any final test-only or formatting fixes**

Only if Step 1-3 required corrections:

```bash
git add crates/inputforge-gui-dx/src crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "test(mapping-list): cover device filter rail"
```

## Self-Review Checklist

- Spec coverage: unnamed rows, device references anywhere, one-row device chips, vJoy badge, sticky Add Mapping, empty clear actions, accessibility, deterministic traversal.
- Red-flag scan: no unresolved markers or delegated comparison language.
- Type consistency: `referenced_devices: Vec<DeviceId>`, `first_vjoy_output: Option<OutputAddress>`, `DeviceChip { id: DeviceId, label: String }`.
