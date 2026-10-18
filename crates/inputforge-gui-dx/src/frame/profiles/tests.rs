use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::mode::Modes;
use inputforge_core::profile::Profile;
use inputforge_core::snapshot::{SnapshotConfig, SnapshotId, SnapshotKind, create};
use inputforge_core::state::{AppState, ProfileOrigin};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, SettingsSnapshot};
use crate::frame::layout::EmptyState;
use crate::frame::profiles::ProfilesPanel;
use crate::frame::profiles::snapshot_drawer::SnapshotDrawer;
use crate::frame::view_state::{MainSurface, PanelSlot, ViewState};
use crate::profiles::{Intent, Mode};
use crate::snapshots::SnapshotRowView;

fn simple_profile(name: &str) -> Profile {
    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    Profile::new(
        name.to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    )
}

fn sample_profiles_context() -> AppState {
    let mut state = AppState::with_profile(simple_profile("Default"));
    state.profile_path = Some(PathBuf::from("C:/Profiles/Default.toml"));
    state.active_profile_origin = Some(ProfileOrigin::Library);
    state
}

#[component]
fn ProfilesHarness() -> Element {
    let app_state = sample_profiles_context();
    let initial_meta = inputforge_app::bridge::prepare(&app_state, None).meta;
    let state = Arc::new(RwLock::new(app_state));
    let (commands, _rx) = mpsc::channel();
    let meta = use_signal(|| initial_meta.clone());
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let main_surface = use_signal(MainSurface::default);
    let editing_mode = use_signal(|| "Default".to_owned());
    let panel_slot = use_signal(|| PanelSlot::Profiles);
    let via_calibration = use_signal(|| false);
    let selected_mapping = use_signal(|| None);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    use_context_provider(|| ViewState {
        main_surface,
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    });
    rsx! { ProfilesPanel {} }
}

#[component]
fn EmptyHarness() -> Element {
    let (commands, _rx) = mpsc::channel();
    crate::test_support::provide_engine(Arc::new(RwLock::new(AppState::new())), commands);
    let view = ViewState {
        main_surface: use_signal(MainSurface::default),
        editing_mode: use_signal(String::new),
        panel_slot: use_signal(PanelSlot::default),
        via_calibration: use_signal(|| false),
        selected_mapping: use_signal(|| None),
    };
    use_context_provider(|| view);
    rsx! { EmptyState {} }
}

fn render_profiles_panel(state: AppState) -> String {
    let _state = state;
    let mut vdom = VirtualDom::new(ProfilesHarness);
    vdom.rebuild_in_place();
    render(&vdom)
}

fn render_no_profile_frame() -> String {
    let mut vdom = VirtualDom::new(EmptyHarness);
    vdom.rebuild_in_place();
    render(&vdom)
}

fn sample_snapshot_context() -> Vec<SnapshotRowView> {
    vec![SnapshotRowView {
        id: sample_snapshot_id(),
        kind: SnapshotKind::Manual,
        kind_label: "Manual".to_owned(),
        label: Some("Before trim".to_owned()),
        time_relative: "12m ago".to_owned(),
        time_absolute: "2026-05-06 22:00 +02:00".to_owned(),
        sort_key: 1,
        pinned: true,
    }]
}

fn sample_session_start_row() -> SnapshotRowView {
    SnapshotRowView {
        id: sample_snapshot_id(),
        kind: SnapshotKind::AutoSessionStart,
        kind_label: "Session start".to_owned(),
        label: None,
        time_relative: "just now".to_owned(),
        time_absolute: "2026-05-07 16:42 +02:00".to_owned(),
        sort_key: 0,
        pinned: false,
    }
}

#[component]
fn SnapshotDrawerHarness(rows: Vec<SnapshotRowView>, open: bool) -> Element {
    let mut app_state = sample_profiles_context();
    app_state.active_snapshot_rows = rows
        .iter()
        .map(|r| inputforge_core::state::ActiveSnapshotRow {
            id: r.id,
            kind: r.kind,
            label: r.label.clone(),
            taken_at: chrono::Utc::now(),
            pinned: r.pinned,
        })
        .collect();
    let initial_meta = inputforge_app::bridge::prepare(&app_state, None).meta;
    let state = Arc::new(RwLock::new(app_state));
    let (commands, _rx) = mpsc::channel();
    let meta = use_signal(|| initial_meta.clone());
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let main_surface = use_signal(MainSurface::default);
    let editing_mode = use_signal(|| "Default".to_owned());
    let panel_slot = use_signal(|| PanelSlot::Profiles);
    let via_calibration = use_signal(|| false);
    let selected_mapping = use_signal(|| None);
    let settings = use_signal(SettingsSnapshot::default);
    let controller = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    let mut view = use_signal(crate::profiles::View::default);
    let owner = use_hook(|| {
        let owner = controller.mount(view);
        let mut prepared = view.peek().clone();
        prepared.snapshots.open = open;
        for (prepared, row) in prepared.snapshots.rows.iter_mut().zip(rows) {
            prepared.row = row;
        }
        view.set(prepared);
        owner
    });
    use_context_provider(|| crate::profiles::Context {
        view,
        owner,
        workflow: 0,
        bindings: controller.bindings(),
    });
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    use_context_provider(|| ViewState {
        main_surface,
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    });

    rsx! {
        SnapshotDrawer {}
    }
}

fn render_snapshot_drawer(rows: Vec<SnapshotRowView>, open: bool) -> String {
    let mut vdom = VirtualDom::new_with_props(
        SnapshotDrawerHarness,
        SnapshotDrawerHarnessProps { rows, open },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

fn sample_snapshot_id() -> SnapshotId {
    let dir = std::env::temp_dir().join(format!(
        "inputforge-gui-snapshot-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.toml");
    simple_profile("Snapshot").save(&path).unwrap();
    let id = create(
        &path,
        SnapshotKind::Manual,
        None,
        &SnapshotConfig::default(),
    )
    .unwrap()
    .unwrap()
    .id;
    let _ = std::fs::remove_dir_all(&dir);
    id
}

#[component]
fn ProfilesHarnessWithMode(mode: Mode) -> Element {
    let app_state = sample_profiles_context();
    let initial_meta = inputforge_app::bridge::prepare(&app_state, None).meta;
    let state = Arc::new(RwLock::new(app_state));
    let (commands, _rx) = mpsc::channel();
    let meta = use_signal(|| initial_meta.clone());
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let main_surface = use_signal(MainSurface::default);
    let editing_mode = use_signal(|| "Default".to_owned());
    let panel_slot = use_signal(|| PanelSlot::Profiles);
    let via_calibration = use_signal(|| false);
    let selected_mapping = use_signal(|| None);
    let controller = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    let mut bindings = controller.bindings();
    bindings.mount = Callback::new(move |view| {
        let owner = controller.mount(view);
        match &mode {
            Mode::New => {
                let _ = controller.dispatch(owner, Intent::New);
            }
            Mode::Open { path } => {
                let request = controller.begin_pick(owner).unwrap();
                controller.finish_pick(&request, Some(path.clone()));
            }
            Mode::Library => {}
        }
        owner
    });
    use_context_provider(|| bindings);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    use_context_provider(|| ViewState {
        main_surface,
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    });
    rsx! { ProfilesPanel {} }
}

fn render_profiles_panel_with_mode(mode: Mode) -> String {
    let mut vdom = VirtualDom::new_with_props(
        ProfilesHarnessWithMode,
        ProfilesHarnessWithModeProps { mode },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

#[test]
fn new_profile_submode_replaces_library_region() {
    let html = render_profiles_panel_with_mode(Mode::New);

    assert!(html.contains("profiles-panel__submode"));
    assert!(html.contains("New profile"));
    assert!(html.contains("profiles-panel__source-group"));
    assert!(html.contains("Blank"));
    assert!(html.contains("Copy active"));
    assert!(html.contains("Copy from library"));
    assert!(html.contains("Open existing file"));
    // Library list header is not rendered while in sub-mode.
    assert!(!html.contains("profile-row__menu-trigger"));
}

#[test]
fn open_choice_submode_replaces_library_region() {
    let mode = Mode::Open {
        path: PathBuf::from("E:/Profiles/external.toml"),
    };
    let html = render_profiles_panel_with_mode(mode);

    assert!(html.contains("profiles-panel__submode"));
    assert!(html.contains("Open profile"));
    assert!(
        html.contains("E:/Profiles/external.toml") || html.contains("E:\\Profiles\\external.toml")
    );
    assert!(html.contains("Load once"));
    assert!(html.contains("Add to library"));
    assert!(!html.contains("profile-row__menu-trigger"));
}

#[test]
fn submode_back_button_renders_with_chevron_text() {
    let html = render_profiles_panel_with_mode(Mode::New);

    assert!(html.contains("profiles-panel__submode-back"));
    assert!(html.contains("Back to library"));
}

#[test]
fn new_profile_source_radios_render_all_four_options() {
    let html = render_profiles_panel_with_mode(Mode::New);

    let radio_count = html.matches("name=\"new-profile-source\"").count();
    assert_eq!(
        radio_count, 4,
        "expected 4 source radios, got {radio_count} in: {html}"
    );
}

#[test]
fn library_filter_renders_inside_library_with_sticky_class() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("profiles-panel__filter"));
    // Filter sits inside the library, not in the panel header.
    let filter_idx = html.find("profiles-panel__filter").unwrap();
    let library_idx = html.find("profiles-panel__library").unwrap();
    assert!(
        library_idx < filter_idx,
        "filter must render inside library region, after .profiles-panel__library opens"
    );
}

#[test]
fn profiles_panel_replaces_placeholder_copy() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("data-testid=\"profile-library\""));
    assert!(!html.contains("Placeholder"));
}

/// Mirrors `panel_slot::tests::panel_header_omits_placeholder_caption`:
/// the slot tab cluster already labels this surface "Profiles", so
/// the panel must NOT restate that label as in-panel heading text.
/// Locks parity with the Devices slot, which dropped its equivalent
/// caption. The assertion targets the literal label between tags
/// (`>Profiles<`); generic `<h2` is intentionally not asserted here
/// because `ProfilesPanel` mounts the destructive-confirm
/// `ProfileDeleteDialog`, whose `DialogTitle` correctly renders an
/// `<h2>Delete profile</h2>` as the modal's accessible title.
#[test]
fn profiles_panel_header_omits_duplicate_title() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(
        !html.contains(">Profiles<"),
        "the literal 'Profiles' label belongs to the slot tab, not the panel"
    );
}

#[test]
fn profiles_panel_drops_panel_level_header() {
    // The two-button panel header was removed; the create/import
    // affordances now live inline in the filter row and as a trailing
    // row in the library list. This test guards against the header
    // creeping back as a regression.
    let html = render_profiles_panel(sample_profiles_context());

    assert!(!html.contains("profiles-panel__header"));
    assert!(!html.contains("profiles-panel__header-actions"));
}

#[test]
fn profiles_filter_row_anchors_open_file_icon_button() {
    // The filter region is now a flex row holding the input plus an
    // Open-file IconButton; the previous panel-level "Open file..."
    // text button is gone.
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("profiles-panel__filter"));
    assert!(html.contains("profiles-panel__open-file"));
    assert!(html.contains("aria-label=\"Open profile from file\""));
    // The icon-button hosts the FolderOpen glyph (currentColor strokes
    // means we look for the path data unique to the Phosphor SVG).
    assert!(html.contains("M32,208V64"));
    // No header-level text button surfaces ">Open file...<" text any
    // more (the no-profile empty-state bar still renders that label,
    // but only when there is no active profile, which is not the case
    // in the default sample context).
    assert!(!html.contains(">Open file...<"));
}

#[test]
fn profiles_library_appends_trailing_create_row() {
    // The "+ New profile" trigger now lives at the end of the library
    // list as a dashed-border button row, replacing the primary header
    // button. Locks both the rendering and its onclick affordance.
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("profile-row profile-row--create"));
    assert!(html.contains("profile-row--create__label"));
    assert!(html.contains(">New profile<"));
    // Order check: the create row is the LAST .profile-row in the list,
    // so its index must be greater than every other profile-row index.
    let create_idx = html
        .find("profile-row--create")
        .expect("create row present");
    let active_idx = html
        .find("profile-row--active")
        .expect("active row present");
    assert!(
        active_idx < create_idx,
        "create row must trail active/inactive rows"
    );
}

#[test]
fn profiles_css_locks_create_row_dashed_contract() {
    // The trailing create row uses .profile-row's shape but a dashed
    // border + transparent surface so it reads as a creator, not a
    // record. Hover/focus raise the border to the strong/focus tones
    // the rest of the system uses for interactive intent.
    let css = include_str!("../../../assets/frame/profiles.css");

    let create_block = css
        .split(".profile-row--create {")
        .nth(1)
        .expect(".profile-row--create rule present")
        .split('}')
        .next()
        .expect(".profile-row--create rule closed");
    assert!(create_block.contains("background: transparent;"));
    assert!(create_block.contains("border-style: dashed;"));
    assert!(create_block.contains("cursor: pointer;"));
}

#[test]
fn active_profile_row_renders_when_library_rows_are_empty() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("Default"));
    assert!(html.contains("profile-row--active"));
    // Spec column contract: name + active pill + mode count + last-edited;
    // path label is intentionally absent.
    assert!(!html.contains("profile-row__path"));
}

#[test]
fn profiles_panel_uses_design_system_components() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("if-button"));
    assert!(html.contains("if-icon-button"));
    assert!(html.contains("if-badge"));
    assert!(html.contains("if-drawer"));
    assert!(html.contains("snapshot-drawer__bar"));
    assert!(html.contains("if-menu"));
    assert!(html.contains("if-menu__trigger"));
    assert!(html.contains("if-menu__item"));
    assert!(html.contains("aria-haspopup=\"true\""));
    assert!(!html.contains("button button--primary"));
    assert!(!html.contains("class=\"button\""));
    assert!(!html.contains("class=\"icon-button\""));
    assert!(!html.contains("class=\"badge\""));
    assert!(!html.contains("profile-row__actions"));
    assert!(!html.contains(">v<"));
}

#[test]
fn profiles_css_uses_flex_layout_and_flush_drawer_contract() {
    let css = include_str!("../../../assets/frame/profiles.css");

    assert!(css.contains(".profiles-panel {\n  display: flex;"));
    assert!(css.contains("flex-direction: column;"));
    assert!(css.contains(".profiles-panel__body {\n  flex: 1 1 auto;"));
    assert!(css.contains(".profile-row {\n  display: flex;"));
    // Snapshot region docks at the bottom of the panel body and feeds
    // the Drawer primitive its open-state size via --if-drawer-size.
    let snapshot_block = css
        .split(".snapshot-drawer {")
        .nth(1)
        .expect("snapshot-drawer rule present")
        .split('}')
        .next()
        .expect("snapshot-drawer rule closed");
    assert!(snapshot_block.contains("--if-drawer-size:"));
    assert!(snapshot_block.contains("margin-top: auto;"));
    // Flush drawer contract: panel fills the slot directly (no
    // negative-margin bleed-out hack); the slot itself is now
    // chrome-only so the panel reaches structural seams without
    // escaping a padding ring.
    assert!(css.contains(".profiles-panel {\n  display: flex;"));
    assert!(css.contains("width: 100%;"));
    assert!(css.contains("height: 100%;"));
    assert!(!css.contains("margin: calc(-1 * var(--space-3));"));
    assert!(!css.contains("width: calc(100% + (2 * var(--space-3)));"));
    assert!(!css.contains("scrollbar-gutter: auto;"));
    assert!(!css.contains("margin-bottom: calc(-1 * var(--space-3));"));
    assert!(!css.contains("grid-template-rows"));
    assert!(!css.contains("grid-row"));
}

#[test]
fn profiles_css_locks_collapsible_drawer_surface_contract() {
    // DESIGN.md §6 "Collapsible Drawer" contract: flips to bg-sunken,
    // 44px header bar, strong-border-top on the bar. The luminance shift
    // is the signal that this is a different region from the list above.
    let css = include_str!("../../../assets/frame/profiles.css");

    let drawer_block = css
        .split(".snapshot-drawer {")
        .nth(1)
        .expect("snapshot-drawer rule present")
        .split('}')
        .next()
        .expect("snapshot-drawer rule closed");
    assert!(
        drawer_block.contains("background: var(--color-bg-sunken);"),
        ".snapshot-drawer must declare bg-sunken per DESIGN.md §6 Collapsible \
         Drawer contract. Found: {drawer_block}"
    );

    let bar_block = css
        .split(".snapshot-drawer__bar {")
        .nth(1)
        .expect("snapshot-drawer__bar rule present")
        .split('}')
        .next()
        .expect("snapshot-drawer__bar rule closed");
    assert!(
        bar_block.contains("min-height: 44px;"),
        ".snapshot-drawer__bar must hold the 44px header height per \
         DESIGN.md §6. Found: {bar_block}"
    );
    assert!(
        bar_block.contains("border-top: 1px solid var(--color-border-strong);"),
        ".snapshot-drawer__bar must use strong-border-top per DESIGN.md §6. \
         Found: {bar_block}"
    );
}

#[test]
fn panel_slot_css_keeps_chrome_only_consumers_own_padding() {
    let css = include_str!("../../../assets/frame/panel_slot.css");

    // Slot itself should not impose padding on consumers.
    let slot_block = css
        .split(".if-panel-slot {")
        .nth(1)
        .expect("panel slot rule present")
        .split('}')
        .next()
        .expect("panel slot rule closed");
    assert!(
        !slot_block.contains("padding:"),
        "panel slot must not impose padding on consumers, found: {slot_block}"
    );

    // Each consumer carries its own breathing-ring padding.
    assert!(css.contains(".if-device-panel {"));
    assert!(css.contains(".if-device-panel--empty {"));
    assert!(css.contains(".if-panel-slot__placeholder {"));
}

#[test]
fn drawer_css_collapses_persistent_and_uses_motion_tokens() {
    let css = include_str!("../../../assets/components/layout/drawer.css");

    // Persistent variant collapses the docked wrapper on the cross-axis
    // when closed and animates with the cockpit-brisk container tokens
    // (DESIGN.md M5: --duration-slow + --easing-standard for >=240ms
    // container enter/exit).
    assert!(css.contains(".if-drawer--persistent"));
    assert!(css.contains("var(--duration-slow)"));
    assert!(css.contains("var(--easing-standard)"));
    // Open state reads --if-drawer-size; closed state collapses to 0.
    assert!(css.contains("max-height: var(--if-drawer-size"));
    assert!(css.contains("max-width: var(--if-drawer-size"));
    assert!(css.contains("max-height: 0;"));
    assert!(css.contains("max-width: 0;"));
    // All four anchors carry their own hairline rule on the inward edge.
    assert!(css.contains(".if-drawer--anchor-bottom > .if-drawer__paper"));
    assert!(css.contains(".if-drawer--anchor-top > .if-drawer__paper"));
    assert!(css.contains(".if-drawer--anchor-left > .if-drawer__paper"));
    assert!(css.contains(".if-drawer--anchor-right > .if-drawer__paper"));
    // Reduced-motion drops the spatial transition to opacity-only.
    assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
}

#[test]
fn no_profile_state_shows_center_explanation_and_panel_actions() {
    let html = render_no_profile_frame();

    assert!(html.contains("No profile loaded"));
    assert!(html.contains("New profile"));
    assert!(html.contains("Open file"));
    assert!(!html.contains("mapping-list"));
}

#[test]
fn snapshot_drawer_renders_empty_state_when_no_snapshots() {
    // Opening the drawer with zero snapshots must not reveal a blank
    // cap of --if-drawer-size. The empty-state placeholder explains
    // that no snapshots exist and how to create one, so the open
    // state is always meaningful regardless of profile state.
    let html = render_snapshot_drawer(Vec::new(), true);

    assert!(html.contains("snapshot-drawer__empty"));
    assert!(html.contains("No snapshots yet"));
    assert!(html.contains("Ctrl+S"));
    // The empty branch replaces row markup; no .snapshot-row article
    // should appear when rows is empty.
    assert!(!html.contains("class=\"snapshot-row\""));
}

#[test]
fn snapshot_drawer_omits_empty_state_when_rows_present() {
    let html = render_snapshot_drawer(sample_snapshot_context(), true);

    assert!(!html.contains("snapshot-drawer__empty"));
    assert!(html.contains("class=\"snapshot-row\""));
}

#[test]
fn snapshot_drawer_caps_open_size_via_viewport_unit() {
    // Percentage --if-drawer-size would not resolve against the
    // snapshot-drawer's flex:0 0 auto parent, leaving the drawer
    // effectively uncapped. Viewport units bypass that resolution
    // problem; this asserts the consumer wires it that way.
    let css = include_str!("../../../assets/frame/profiles.css");
    let snapshot_block = css
        .split(".snapshot-drawer {")
        .nth(1)
        .expect("snapshot-drawer rule present")
        .split('}')
        .next()
        .expect("snapshot-drawer rule closed");
    assert!(
        snapshot_block.contains("--if-drawer-size: 40vh;"),
        "snapshot-drawer must set --if-drawer-size with a viewport-relative \
         unit so the Drawer's max-height resolves; got: {snapshot_block}"
    );
}

#[test]
fn snapshot_row_renders_two_line_layout_with_icon_only_actions() {
    let html = render_snapshot_drawer(sample_snapshot_context(), true);

    // Two-line structure: leading kind-icon, primary line carrying
    // the strong-label (with `title` attribute for full-text tooltip
    // on truncation), secondary line carrying the relative time and
    // the icon-only action pair (Restore + Ghost trash for Delete).
    assert!(html.contains("class=\"snapshot-row__kind-icon\""));
    assert!(html.contains("class=\"snapshot-row__primary\""));
    assert!(html.contains("class=\"snapshot-row__secondary\""));
    assert!(html.contains("class=\"snapshot-row__label\""));
    assert!(html.contains("title=\"Before trim\""));
    assert!(html.contains("<time"));
    assert!(html.contains("datetime=\"2026-05-06 22:00 +02:00\""));
    assert!(html.contains("title=\"2026-05-06 22:00 +02:00\""));
    assert!(html.contains("12m ago"));
    // The old single-line grouping is retired.
    assert!(!html.contains("snapshot-row__title"));
    // Restore is now an icon-only Ghost IconButton with the
    // ClockCounterClockwise glyph (Phosphor "restore from history"),
    // aria-label "Restore snapshot", and the .snapshot-row__restore
    // class so CSS can apply the always-visible primary tint on
    // hover. Delete is a Ghost trash IconButton with aria-label
    // "Delete snapshot" behind the .snapshot-row__delete hover-reveal
    // class.
    assert!(html.contains("snapshot-row__restore"));
    assert!(html.contains("aria-label=\"Restore snapshot\""));
    assert!(html.contains("snapshot-row__delete"));
    assert!(html.contains("aria-label=\"Delete snapshot\""));
    // No textual "Restore" button anywhere in the row markup; the
    // icon carries the affordance now (the aria-label is the
    // accessible name for screen readers).
    assert!(!html.contains(">Restore<"));
    // Neither action button uses the danger variant; the destructive
    // read comes from the ghost-on-error-tint CSS state, not from a
    // saturated red surface.
    assert!(!html.contains("if-icon-button--danger"));
}

#[test]
fn snapshot_row_disables_delete_on_session_start() {
    // Session start is the recovery anchor. Deleting it strands the
    // user's "go back to where I was when I opened the app" lifeline.
    // Lock that the Delete affordance is rendered disabled on this row.
    let html = render_snapshot_drawer(vec![sample_session_start_row()], true);

    // The Delete IconButton stays in the markup so the user can see
    // the affordance and reason about it; the disabled attribute and
    // CSS opacity-0.35 make it non-interactive.
    assert!(html.contains("snapshot-row__delete"));
    // The Trash IconButton renders a `<button ... disabled>` for this
    // row. Walk the row's actions block to confirm the disabled
    // attribute is on the delete button.
    let delete_marker =
        "class=\"if-icon-button if-icon-button--ghost if-icon-button--sm snapshot-row__delete\"";
    let after_class = html
        .split(delete_marker)
        .nth(1)
        .expect("session start row must render the snapshot-row__delete button");
    let inside_tag = after_class
        .split('>')
        .next()
        .expect("button tag must close");
    assert!(
        inside_tag.contains("disabled"),
        "session-start row's Delete must be disabled; got tag: {inside_tag}"
    );
}

#[test]
fn snapshot_row_label_never_leaks_hid_hash() {
    // Regression check on the projection contract: the row's user-
    // facing label must never render the raw 32-char SDL HID hash.
    // The bulk-map label producer resolves the source DeviceId to a
    // display name (alias, hardware name, or id-string fallback)
    // before formatting; this test pins that the renderer faithfully
    // displays whatever string came in, and that no test fixture or
    // production projection would smuggle the hash through.
    let html = render_snapshot_drawer(sample_snapshot_context(), true);
    assert!(!html.contains("030037c344330000f483000000000000"));
}

#[test]
fn drawer_header_uses_sibling_toggle_and_snapshot_now_button() {
    let html = render_snapshot_drawer(sample_snapshot_context(), true);

    // Toggle bar is a sibling of the Drawer, both inside the
    // snapshot-drawer section. The toggle button carries
    // aria-expanded so CSS can rotate the chevron in place of swapping
    // glyphs, and aria-controls points at the body inside the Drawer.
    assert!(html.contains("class=\"snapshot-drawer__bar\""));
    assert!(html.contains("snapshot-drawer__toggle"));
    assert!(html.contains("aria-expanded=\"true\""));
    assert!(html.contains("aria-controls=\"snapshot-drawer-body\""));
    assert!(html.contains("aria-label=\"Snapshot now\""));
    // The Drawer primitive labels its Paper region by the toggle's id.
    assert!(html.contains("aria-labelledby=\"snapshot-drawer-bar-title\""));
    // Rule out a regression where the Button primitive is nested inside
    // the toggle button (the toggle is now a raw <button>, not a
    // wrapping Button).
    assert!(!html.contains("<button class=\"snapshot-drawer__toggle\"><button"));
}

#[test]
fn profiles_surface_never_renders_mapping_counts() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(!html.contains("mapping"));
    assert!(!html.contains("mappings"));
}

#[test]
fn drawer_is_panel_scoped_not_global_drawer() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("if-drawer"));
    assert!(html.contains("snapshot-drawer"));
    assert!(!html.contains("app-global-drawer"));
}
