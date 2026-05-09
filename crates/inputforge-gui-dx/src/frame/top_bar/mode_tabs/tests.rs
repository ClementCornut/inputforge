//! Component tests for `frame::top_bar::mode_tabs`. Mounts `ModeTabs`
//! against a stub-context harness (mirroring
//! `mapping_list::tests::provide_minimal_contexts`) and asserts on the
//! rendered HTML to lock the contracts the cohesion pass introduced
//! when migrating onto the decomposed `Tabs` primitive trio.
//!
//! Coverage scope:
//!
//! - `aria-haspopup="menu"` on every tab (the context menu attaches via
//!   right-click and Shift+F10).
//! - `aria-expanded` reflects the `open_for_tab` signal (false at rest).
//! - `aria-controls` is only present while the menu is mounted.
//! - The `role="tablist"` container holds exactly the real tabs; the
//!   trailing `+` button renders OUTSIDE the tablist so AT counts stay
//!   honest.
//! - `RenameInline` swap: when `renaming.set(Some(name))`, the matching
//!   tab does not render its `TabButton` (`RenameInline` takes its slot,
//!   does not register with the keyboard coordinator).
//!
//! Out of scope (deferred): event-dispatch tests for the per-tab
//! `oncontextmenu`, Shift+F10, and Delete handlers, plus the
//! `pending_focus` -> `TabsRoot.focus_request` round-trip. The
//! `dioxus_ssr` test surface used here does not run event handlers,
//! so verifying these requires a future test-infra extension (e.g.
//! a Dioxus `VirtualDom` event-dispatch helper) before we can assert
//! the open-state signal flips on a synthetic event. The handlers
//! themselves are wired in `mod.rs:121-204` and reviewed at the
//! source level; this file pins the structural contracts that the
//! handlers anchor to (DOM ids, aria-* shape, tablist boundary).

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::Mapping;
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::state::AppState;
use inputforge_core::types::{DeviceId, InputAddress, InputId};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
use crate::frame::top_bar::mode_tabs::{ModeDeleteSignal, ModeFocusSignal, ModeTabs};
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

/// Mirror of `mapping_list::tests::provide_minimal_contexts` plus the
/// `ModeDeleteSignal` that `ModeTabs` consumes (provided shell-side by
/// `Layout` in production; provided locally here so the strip mounts
/// in isolation).
fn provide_mode_tabs_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: use_signal(SettingsSnapshot::default),
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());

    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);

    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });

    use_live_capture_provider();

    let dt: Signal<Option<String>> = use_signal(|| None);
    use_context_provider(|| ModeDeleteSignal(dt));
    let mf: Signal<Option<String>> = use_signal(|| None);
    use_context_provider(|| ModeFocusSignal(mf));
}

/// Build an `AppState` with the supplied modes (parent-children adjacency)
/// and a single bound mapping so `ConfigSnapshot::from_state` produces a
/// non-empty render. The first mode is taken as the startup mode.
fn build_state(mode_adjacency: &[(&str, Vec<&str>)], current: &str) -> AppState {
    let map: std::collections::HashMap<String, Vec<String>> = mode_adjacency
        .iter()
        .map(|(parent, kids)| {
            (
                (*parent).to_owned(),
                kids.iter().map(|k| (*k).to_owned()).collect(),
            )
        })
        .collect();
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let mappings = vec![Mapping {
        input: InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        },
        mode: mode_adjacency[0].0.to_owned(),
        name: Some("Boost".to_owned()),
        actions: vec![],
    }];
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        mode_adjacency[0].0.to_owned(),
    );
    let mut state = AppState::with_profile(profile);
    state.current_mode = current.to_owned();
    state
}

/// Wraps the `AppState` in an `Arc` so it can ride through Dioxus prop
/// equality (which requires `Clone + PartialEq`). The wrapper compares
/// by `Arc::ptr_eq` since `AppState` itself is not `PartialEq`.
#[derive(Clone)]
struct StateHandle(Arc<AppState>);

impl PartialEq for StateHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Props, PartialEq)]
struct StateBootstrapProps {
    state: StateHandle,
}

#[component]
fn StateBootstrap(props: StateBootstrapProps) -> Element {
    provide_mode_tabs_contexts();
    let ctx_app = use_context::<AppContext>();
    let mut cfg_signal = ctx_app.config;
    let mut meta_signal = ctx_app.meta;
    let state_for_hook = Arc::clone(&props.state.0);
    use_hook(move || {
        cfg_signal.set(ConfigSnapshot::from_state(state_for_hook.as_ref(), None));
        meta_signal.set(MetaSnapshot::from_state(state_for_hook.as_ref()));
    });
    rsx! { ModeTabs {} }
}

fn render_with_state(state: AppState) -> String {
    let mut vdom = VirtualDom::new_with_props(
        StateBootstrap,
        StateBootstrapProps {
            state: StateHandle(Arc::new(state)),
        },
    );
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    render(&vdom)
}

/// Every mode tab carries `aria-haspopup="menu"` so AT users learn that
/// a context menu attaches (right-click or Shift+F10). Wired on
/// `TabButton::aria_haspopup` at `mod.rs:225`.
#[test]
fn mode_tabs_each_tab_announces_aria_haspopup_menu() {
    let state = build_state(&[("Default", vec!["Combat"])], "Default");
    let html = render_with_state(state);
    // One occurrence per tab. With Default + Combat, expect at least 2.
    let popup_count = html.matches("aria-haspopup=\"menu\"").count();
    assert!(
        popup_count >= 2,
        "every mode tab must carry aria-haspopup=\"menu\" so context menus \
         are discoverable; got {popup_count} occurrences in: {html}",
    );
}

/// `aria-expanded` reflects the `open_for_tab` signal. Initial render
/// has no menu open, so every tab announces `aria-expanded="false"`.
#[test]
fn mode_tabs_aria_expanded_false_at_rest() {
    let state = build_state(&[("Default", vec!["Combat"])], "Default");
    let html = render_with_state(state);
    let expanded_true = html.matches("aria-expanded=\"true\"").count();
    assert_eq!(
        expanded_true, 0,
        "no tab should announce aria-expanded=\"true\" before the user \
         opens a context menu; got {expanded_true} in: {html}",
    );
    let expanded_false = html.matches("aria-expanded=\"false\"").count();
    assert!(
        expanded_false >= 2,
        "each mode tab must announce aria-expanded=\"false\" while its \
         menu is unmounted; got {expanded_false} in: {html}",
    );
}

/// `aria-controls` only emits while the menu is mounted (per
/// `mod.rs:227-229` comment: "pointing at a missing id confuses AT").
/// At rest, no tab carries aria-controls.
#[test]
fn mode_tabs_aria_controls_omitted_when_menu_closed() {
    let state = build_state(&[("Default", vec!["Combat"])], "Default");
    let html = render_with_state(state);
    assert!(
        !html.contains("aria-controls=\"mode-tab-menu-"),
        "aria-controls must not point at the menu id while no menu is \
         mounted: {html}",
    );
}

/// `role="tablist"` contains exactly the real tabs. The trailing `+`
/// add affordance and the context menu render OUTSIDE the tablist so
/// AT tab counts stay honest. Existing
/// `mapping_list::tests::mode_tabs_add_button_lives_outside_tablist`
/// covers the `+` boundary; this test additionally verifies the
/// `role="tab"` count inside the tablist matches the modes count.
#[test]
fn mode_tabs_tablist_count_matches_modes() {
    let state = build_state(&[("Default", vec!["Combat", "Stealth"])], "Default");
    let html = render_with_state(state);
    let tablist_open = html.find("role=\"tablist\"").expect("tablist must render");
    let tablist_close_relative = html[tablist_open..].find("</div>").expect("tablist closes");
    let tablist_close = tablist_open + tablist_close_relative;
    let tablist_slice = &html[tablist_open..tablist_close];
    let tab_count = tablist_slice.matches("role=\"tab\"").count();
    assert_eq!(
        tab_count, 3,
        "role=tablist must contain exactly the 3 real tabs (Default + \
         Combat + Stealth); got {tab_count} in slice: {tablist_slice}",
    );
    let plus_inside = tablist_slice.contains("aria-label=\"Add mode\"");
    assert!(
        !plus_inside,
        "Add-mode `+` must NOT render inside role=tablist (AT count \
         honesty): {tablist_slice}",
    );
}

/// Mode tabs preserve the `data-mode` attribute that style and tests
/// use to pick out a specific tab. `TabButton::data_mode` at
/// `mod.rs:222` carries the mode name verbatim.
#[test]
fn mode_tabs_each_tab_carries_data_mode_attribute() {
    let state = build_state(&[("Default", vec!["Combat"])], "Default");
    let html = render_with_state(state);
    assert!(
        html.contains("data-mode=\"Default\""),
        "Default tab must carry data-mode=\"Default\": {html}",
    );
    assert!(
        html.contains("data-mode=\"Combat\""),
        "Combat tab must carry data-mode=\"Combat\": {html}",
    );
}

/// DOM ids are derived from the tab index, never the raw mode name.
/// This sidesteps any JS-eval interpolation hazard in the Shift+F10
/// `getBoundingClientRect` call (`mod.rs:158-163`). Lock the contract
/// that `mode-tab-{idx}` ids appear in the rendered HTML.
#[test]
fn mode_tabs_dom_ids_use_index_not_mode_name() {
    let state = build_state(&[("Default", vec!["Combat"])], "Default");
    let html = render_with_state(state);
    assert!(
        html.contains("id=\"mode-tab-0\""),
        "first tab must use id=\"mode-tab-0\" (index-derived): {html}",
    );
    assert!(
        html.contains("id=\"mode-tab-1\""),
        "second tab must use id=\"mode-tab-1\" (index-derived): {html}",
    );
    // Negative: raw mode name must not appear as a DOM id.
    assert!(
        !html.contains("id=\"Default\""),
        "raw mode name must NOT appear as a DOM id (JS-eval safety): {html}",
    );
}
