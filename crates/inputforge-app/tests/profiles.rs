use dioxus::prelude::*;
use inputforge_app::profiles::Controller;
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    profile::{Profile, ProfileId},
    state::{AppState, ProfileLibraryRow, ProfileOrigin},
};
use inputforge_gui_dx::profiles::{Intent, Mode, Source, View};
use parking_lot::RwLock;
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};

fn profile(name: &str) -> Profile {
    Profile::new(
        name.into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    )
}
fn state() -> AppState {
    let mut s = AppState::with_profile(profile("Alpha"));
    s.profile_path = Some("/profiles/Alpha.toml".into());
    s.active_profile_origin = Some(ProfileOrigin::Library);
    for name in ["Alpha", "Bravo"] {
        s.profile_library_rows.push(ProfileLibraryRow {
            name: name.into(),
            path: format!("/profiles/{name}.toml").into(),
            profile_id: Some(if name == "Alpha" {
                s.active_profile.as_ref().unwrap().id().clone()
            } else {
                ProfileId::new()
            }),
            origin: ProfileOrigin::Library,
            is_active: name == "Alpha",
            mode_count: 1,
            last_edited_at: None,
        });
    }
    s
}
type Check =
    fn(&Controller, u64, Signal<View>, Arc<RwLock<AppState>>, mpsc::Receiver<EngineCommand>);
type Mounted = (Controller, u64, Signal<View>);
#[derive(Clone)]
struct Props {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    mounted: std::rc::Rc<std::cell::RefCell<Option<Mounted>>>,
}
fn root(props: Props) -> Element {
    let c = use_hook(|| Controller::new(props.state, props.commands));
    let view = use_signal(View::default);
    let owner = use_hook(|| c.mount(view));
    *props.mounted.borrow_mut() = Some((c, owner, view));
    rsx! {div {}}
}
fn check(connected: bool, test: Check) {
    let state = Arc::new(RwLock::new(state()));
    let (commands, rx) = mpsc::channel();
    let rx = if connected {
        rx
    } else {
        drop(rx);
        mpsc::channel().1
    };
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mut dom = VirtualDom::new_with_props(
        root,
        Props {
            state: Arc::clone(&state),
            commands,
            mounted: std::rc::Rc::clone(&mounted),
        },
    );
    dom.rebuild_in_place();
    let (controller, owner, view) = mounted.borrow().as_ref().unwrap().clone();
    // Assertions run outside render: Dioxus catches render panics for its error boundary.
    dom.in_scope(ScopeId::ROOT, || test(&controller, owner, view, state, rx));
}

#[test]
fn failed_create_retains_form_and_reports_delivery_failure() {
    check(false, |c, o, v, _, _| {
        let _ = c.dispatch(o, Intent::New);
        let _ = c.dispatch(o, Intent::Name(" Example ".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(v.peek().mode, Mode::New);
        assert_eq!(v.peek().name, " Example ");
        assert!(v.peek().error.is_some());
    });
}
#[test]
fn names_are_trimmed_and_collisions_include_case_and_sanitized_filenames() {
    check(true, |c, o, v, s, rx| {
        let _ = c.dispatch(o, Intent::New);
        for invalid in ["  ", "bad/name", "alpha", "BRAVO"] {
            let _ = c.dispatch(o, Intent::Name(invalid.into()));
            let _ = c.dispatch(o, Intent::Create);
            assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
            assert!(v.peek().error.is_some());
            assert_eq!(v.peek().mode, Mode::New);
        }
        s.write().profile_library_rows[1].name = "A.B".into();
        let _ = c.dispatch(o, Intent::Name("A_B".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        let _ = c.dispatch(o, Intent::Name(" Example ".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::CreateProfile {
                name: "Example".into()
            }
        );
        assert_eq!(v.peek().mode, Mode::Library);
    });
}
#[test]
fn case_only_rename_and_external_namespace_use_captured_path() {
    check(true, |c, o, v, s, rx| {
        s.write().active_profile = Some(profile("Alpha"));
        s.write().profile_path = Some("/external/Alpha.toml".into());
        s.write().active_profile_origin = Some(ProfileOrigin::External);
        c.refresh(o);
        let target = v
            .peek()
            .rows
            .iter()
            .find(|r| r.row.id == "/profiles/Alpha.toml")
            .unwrap()
            .target
            .clone();
        let _ = c.dispatch(o, Intent::BeginRename(target));
        let _ = c.dispatch(o, Intent::Name(" ALPHA ".into()));
        let _ = c.dispatch(o, Intent::Rename);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::RenameProfile {
                old_name: "Alpha".into(),
                new_name: "ALPHA".into()
            }
        );
        assert!(v.peek().rename.is_none());
    });
}
#[test]
fn failed_rename_and_delete_keep_drafts_and_confirmation() {
    check(false, |c, o, v, _, _| {
        let target = v.peek().rows[0].target.clone();
        let _ = c.dispatch(o, Intent::BeginRename(target.clone()));
        let _ = c.dispatch(o, Intent::Name("Renamed".into()));
        let _ = c.dispatch(o, Intent::Rename);
        assert_eq!(v.peek().name, "Renamed");
        assert!(v.peek().rename.is_some());
        assert!(v.peek().error.is_some());
        let _ = c.dispatch(o, Intent::CancelRename);
        let _ = c.dispatch(o, Intent::BeginDelete(target));
        let _ = c.dispatch(o, Intent::Delete);
        assert!(v.peek().delete.is_some());
        assert!(v.peek().deleting_active);
        assert!(v.peek().error.is_some());
    });
}
#[test]
fn replacement_rejects_captured_row_actions_and_retained_forms() {
    check(true, |c, o, v, s, rx| {
        let target = v.peek().rows[0].target.clone();
        let _ = c.dispatch(o, Intent::BeginRename(target.clone()));
        s.write().active_profile = Some(profile("Alpha"));
        let _ = c.dispatch(o, Intent::Name("Renamed".into()));
        let _ = c.dispatch(o, Intent::Rename);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert!(v.peek().rename.is_some());
        assert!(v.peek().error.is_some());
        let _ = c.dispatch(o, Intent::BeginDelete(target.clone()));
        assert!(v.peek().delete.is_none());
        let _ = c.dispatch(o, Intent::Open(target));
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}
#[test]
fn inactive_same_name_replacement_rejects_delete_and_copy() {
    check(true, |c, o, v, s, rx| {
        let target = v
            .peek()
            .rows
            .iter()
            .find(|r| r.row.name == "Bravo")
            .unwrap()
            .target
            .clone();
        let _ = c.dispatch(o, Intent::BeginDelete(target.clone()));
        s.write().profile_library_rows[1].profile_id = Some(ProfileId::new());
        let _ = c.dispatch(o, Intent::Delete);
        assert!(v.peek().delete.is_some());
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        let _ = c.dispatch(o, Intent::Duplicate(target.clone()));
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        let _ = c.dispatch(o, Intent::New);
        let _ = c.dispatch(o, Intent::Source(Source::Library(Some(target))));
        let _ = c.dispatch(o, Intent::Name("Copy".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}
#[test]
fn filtering_keeps_active_and_selection_uses_native_path() {
    check(true, |c, o, v, _, rx| {
        let _ = c.dispatch(o, Intent::Filter("BrAv".into()));
        assert_eq!(v.peek().rows.len(), 2);
        let _ = c.dispatch(o, Intent::Filter("missing".into()));
        assert_eq!(v.peek().rows.len(), 1);
        assert_eq!(v.peek().rows[0].row.name, "Alpha");
        assert!(v.peek().filtered_empty);
        let _ = c.dispatch(o, Intent::Filter(String::new()));
        let t = v.peek().rows[1].target.clone();
        let _ = c.dispatch(o, Intent::Open(t));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::LoadProfile("/profiles/Bravo.toml".into())
        );
    });
}
#[test]
fn picker_cancel_and_in_library_vs_external_completion() {
    check(true, |c, o, v, _, rx| {
        let _ = c.dispatch(o, Intent::New);
        let request = c.begin_pick(o).unwrap();
        c.finish_pick(&request, None);
        assert_eq!(v.peek().mode, Mode::New);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        let request = c.begin_pick(o).unwrap();
        let path = inputforge_core::settings::AppSettings::profiles_dir().join("Known.toml");
        c.finish_pick(&request, Some(path.clone()));
        assert_eq!(rx.try_recv().unwrap(), EngineCommand::LoadProfile(path));
        assert_eq!(v.peek().mode, Mode::Library);
        let request = c.begin_pick(o).unwrap();
        c.finish_pick(&request, Some("/external/Imported.toml".into()));
        assert_eq!(v.peek().name, "Imported");
        assert_eq!(
            v.peek().mode,
            Mode::Open {
                path: PathBuf::from("/external/Imported.toml")
            }
        );
        let _ = c.dispatch(o, Intent::LoadOnce);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::LoadExternalProfileOnce("/external/Imported.toml".into())
        );
    });
}
#[test]
fn obsolete_picker_after_close_newer_request_unmount_or_profile_replacement_is_ignored() {
    check(true, |c, o, v, s, rx| {
        let old = c.begin_pick(o).unwrap();
        let _ = c.dispatch(o, Intent::Back);
        c.finish_pick(&old, Some("/external/Old.toml".into()));
        assert_eq!(v.peek().mode, Mode::Library);
        let old = c.begin_pick(o).unwrap();
        let latest = c.begin_pick(o).unwrap();
        c.finish_pick(&old, Some("/external/Old.toml".into()));
        assert_eq!(v.peek().mode, Mode::Library);
        s.write().active_profile = Some(profile("Alpha"));
        c.finish_pick(&latest, Some("/external/Old.toml".into()));
        assert_eq!(v.peek().mode, Mode::Library);
        let old = c.begin_pick(o).unwrap();
        c.unmount(o);
        let fresh = c.mount(v);
        c.finish_pick(&old, Some("/external/Old.toml".into()));
        assert_eq!(v.peek().mode, Mode::Library);
        let _ = c.dispatch(o, Intent::New);
        assert_eq!(v.peek().mode, Mode::Library);
        let _ = c.dispatch(fresh, Intent::New);
        assert_eq!(v.peek().mode, Mode::New);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}
#[test]
fn failed_import_load_and_in_library_picker_do_not_close_form() {
    check(false, |c, o, v, _, _| {
        let req = c.begin_pick(o).unwrap();
        c.finish_pick(&req, Some("/external/Alpha.toml".into()));
        for intent in [Intent::LoadOnce, Intent::Add] {
            let _ = c.dispatch(o, intent);
            assert!(matches!(v.peek().mode, Mode::Open { .. }));
            assert!(v.peek().error.is_some());
            assert_eq!(v.peek().name, "Alpha");
        }
        let _ = c.dispatch(o, Intent::Name("Imported".into()));
        let _ = c.dispatch(o, Intent::Add);
        assert!(matches!(v.peek().mode, Mode::Open { .. }));
        assert!(v.peek().error.is_some());
        let req = c.begin_pick(o).unwrap();
        c.finish_pick(
            &req,
            Some(inputforge_core::settings::AppSettings::profiles_dir().join("Known.toml")),
        );
        assert!(matches!(v.peek().mode, Mode::Open { .. }));
        assert!(v.peek().error.is_some());
    });
}
#[test]
fn copy_import_duplicate_reveal_add_and_delete_emit_existing_engine_commands() {
    check(true, |c, o, v, _, rx| {
        let target = v.peek().rows[1].target.clone();
        let _ = c.dispatch(o, Intent::Duplicate(target.clone()));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::DuplicateProfile {
                source_path: "/profiles/Bravo.toml".into(),
                name: "Bravo Copy".into()
            }
        );
        let _ = c.dispatch(o, Intent::Reveal(target.clone()));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::RevealProfile {
                path: "/profiles/Bravo.toml".into()
            }
        );
        let _ = c.dispatch(o, Intent::New);
        let _ = c.dispatch(o, Intent::Source(Source::Library(Some(target.clone()))));
        let _ = c.dispatch(o, Intent::Name("Copy".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::DuplicateProfile {
                source_path: "/profiles/Bravo.toml".into(),
                name: "Copy".into()
            }
        );
        let _ = c.dispatch(o, Intent::BeginDelete(target));
        let _ = c.dispatch(o, Intent::Delete);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::DeleteProfile {
                name: "Bravo".into()
            }
        );
        assert!(v.peek().delete.is_none());
        let req = c.begin_pick(o).unwrap();
        c.finish_pick(&req, Some("/external/Import.toml".into()));
        let _ = c.dispatch(o, Intent::Name("Imported".into()));
        let _ = c.dispatch(o, Intent::Add);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::AddExternalProfileToLibrary {
                path: "/external/Import.toml".into(),
                name: "Imported".into()
            }
        );
    });
}

#[test]
fn obsolete_form_handlers_cannot_submit_or_edit_a_new_workflow() {
    check(true, |c, o, v, _, rx| {
        let _ = c.dispatch(o, Intent::New);
        let captured = v.peek().workflow;
        let _ = c.dispatch(o, Intent::Back);
        let _ = c.dispatch(o, Intent::New);
        let _ = c.dispatch(o, Intent::Name("Current draft".into()));
        assert!(
            !c.bindings()
                .dispatch
                .call((o, captured, Intent::Name("Obsolete".into())))
        );
        assert!(!c.bindings().dispatch.call((o, captured, Intent::Create)));
        assert_eq!(v.peek().name, "Current draft");
        assert_eq!(v.peek().mode, Mode::New);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}

#[cfg(unix)]
#[test]
fn external_row_keeps_non_utf8_native_path_for_reveal_and_import() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    check(true, |c, o, v, s, rx| {
        let path = PathBuf::from(OsString::from_vec(b"/external/\xff.toml".to_vec()));
        s.write().active_profile = Some(profile("External"));
        s.write().profile_path = Some(path.clone());
        s.write().active_profile_origin = Some(ProfileOrigin::External);
        c.refresh(o);
        let t = v
            .peek()
            .rows
            .iter()
            .find(|r| r.row.origin == inputforge_gui_dx::snapshots::ProfileRowOrigin::External)
            .expect("external native path must stay visible")
            .target
            .clone();
        let _ = c.dispatch(o, Intent::Reveal(t.clone()));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::RevealProfile { path: path.clone() }
        );
        let _ = c.dispatch(o, Intent::AddRow(t));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::AddExternalProfileToLibrary {
                path,
                name: "External".into()
            }
        );
    });
}

#[test]
fn unchanged_rename_and_cancelled_delete_send_nothing() {
    check(true, |c, o, v, _, rx| {
        let t = v.peek().rows[0].target.clone();
        let _ = c.dispatch(o, Intent::BeginRename(t.clone()));
        let _ = c.dispatch(o, Intent::Name(" Alpha ".into()));
        let _ = c.dispatch(o, Intent::Rename);
        assert!(v.peek().rename.is_none());
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        let _ = c.dispatch(o, Intent::BeginDelete(t));
        let _ = c.dispatch(o, Intent::CancelDelete);
        assert!(v.peek().delete.is_none());
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}
#[test]
fn copy_active_uses_current_native_path_and_missing_sources_stay_in_form() {
    check(true, |c, o, v, s, rx| {
        let _ = c.dispatch(o, Intent::New);
        let _ = c.dispatch(o, Intent::Name("Copy".into()));
        let _ = c.dispatch(o, Intent::Source(Source::Active));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::DuplicateProfile {
                source_path: "/profiles/Alpha.toml".into(),
                name: "Copy".into()
            }
        );
        s.write().profile_path = None;
        s.write().active_profile = None;
        for source in [Source::Active, Source::Library(None), Source::File] {
            let _ = c.dispatch(o, Intent::New);
            let _ = c.dispatch(o, Intent::Source(source));
            let _ = c.dispatch(o, Intent::Create);
            assert_eq!(v.peek().mode, Mode::New);
            assert!(!v.peek().can_create);
            assert!(v.peek().error.is_some());
            assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        }
    });
}
#[test]
fn failed_menu_delivery_reports_rejection_and_top_bar_navigation_is_app_owned() {
    check(false, |c, o, v, _, _| {
        let t = v.peek().rows[1].target.clone();
        for intent in [
            Intent::Open(t.clone()),
            Intent::Duplicate(t.clone()),
            Intent::Reveal(t),
        ] {
            assert!(!c.dispatch(o, intent));
            assert!(v.peek().error.is_some());
            assert_eq!(v.peek().mode, Mode::Library);
        }
        let panel = Signal::new(inputforge_gui_dx::PanelSlot::None);
        c.bindings().open_panel.call(panel);
        assert_eq!(*panel.peek(), inputforge_gui_dx::PanelSlot::Profiles);
    });
}

#[test]
fn obsolete_picker_is_ignored_even_when_engine_read_is_contended() {
    check(true, |c, o, v, s, rx| {
        let request = c.begin_pick(o).unwrap();
        let _ = c.dispatch(o, Intent::Back);
        let _ = c.dispatch(o, Intent::New);
        let state_guard = s.write();
        c.finish_pick(&request, Some("/external/Obsolete.toml".into()));
        assert!(v.peek().error.is_none());
        assert_eq!(v.peek().mode, Mode::New);
        drop(state_guard);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}

#[test]
fn active_library_file_replacement_rejects_retained_rename_and_delete() {
    check(true, |c, o, v, s, rx| {
        let old = v.peek().rows[0].target.clone();
        let _ = c.dispatch(o, Intent::BeginRename(old));
        s.write().profile_library_rows[0].profile_id = Some(ProfileId::new());
        let _ = c.dispatch(o, Intent::Name("Renamed".into()));
        let _ = c.dispatch(o, Intent::Rename);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert!(v.peek().rename.is_some());
        assert!(v.peek().error.is_some());
        let _ = c.dispatch(o, Intent::CancelRename);
        c.refresh(o);
        let current = v.peek().rows[0].target.clone();
        let _ = c.dispatch(o, Intent::BeginDelete(current));
        s.write().profile_library_rows[0].profile_id = Some(ProfileId::new());
        let _ = c.dispatch(o, Intent::Delete);
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert!(v.peek().delete.is_some());
        assert!(v.peek().error.is_some());
    });
}
#[cfg(unix)]
#[test]
fn distinct_non_utf8_library_paths_keep_separate_rows_and_copy_source() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    check(true, |c, o, v, s, rx| {
        let first = PathBuf::from(OsString::from_vec(b"/profiles/\xff.toml".to_vec()));
        let second = PathBuf::from(OsString::from_vec(b"/profiles/\xfe.toml".to_vec()));
        assert_eq!(first.display().to_string(), second.display().to_string());
        {
            let mut state = s.write();
            state.profile_path = None;
            state.active_profile = None;
            for (row, path) in state
                .profile_library_rows
                .iter_mut()
                .zip([first.clone(), second.clone()])
            {
                row.path = path;
                row.name = "Same display".into();
                row.is_active = false;
            }
        }
        c.refresh(o);
        assert_eq!(v.peek().rows[0].target.path, first);
        assert_eq!(v.peek().rows[1].target.path, second);
        assert_ne!(v.peek().rows[0].target.key, v.peek().rows[1].target.key);
        let target = v.peek().rows[1].target.clone();
        let _ = c.dispatch(o, Intent::Open(target.clone()));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::LoadProfile(second.clone())
        );
        let _ = c.dispatch(o, Intent::Reveal(target));
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::RevealProfile {
                path: second.clone()
            }
        );
        let _ = c.dispatch(o, Intent::New);
        let choices = v.peek().library.clone();
        let _ = c.dispatch(
            o,
            Intent::LibrarySource {
                key: choices[1].key.clone(),
                choices,
            },
        );
        let _ = c.dispatch(o, Intent::Name("Copy".into()));
        let _ = c.dispatch(o, Intent::Create);
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::DuplicateProfile {
                source_path: second,
                name: "Copy".into()
            }
        );
    });
}

#[test]
fn projection_pins_active_and_sorts_inactive_before_filtering() {
    check(true, |c, o, v, s, _| {
        let mut state = s.write();
        let mut zulu = state.profile_library_rows[0].clone();
        zulu.name = "Zulu".into();
        zulu.path = "/profiles/Zulu.toml".into();
        zulu.is_active = false;
        state.profile_library_rows.push(zulu);
        state.profile_library_rows.swap(0, 2);
        state.profile_path = Some("/profiles/Bravo.toml".into());
        state.active_profile = Some(profile("Bravo"));
        drop(state);
        c.refresh(o);
        assert_eq!(
            v.peek()
                .rows
                .iter()
                .map(|r| r.row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Bravo", "Alpha", "Zulu"]
        );
        let _ = c.dispatch(o, Intent::Filter(" alp ".into()));
        assert_eq!(
            v.peek()
                .rows
                .iter()
                .map(|r| r.row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Bravo", "Alpha"]
        );
    });
}

#[test]
fn synthesized_profile_row_keeps_existing_bridge_metadata_labels_across_events() {
    check(true, |c, o, v, s, _| {
        let mut state = s.write();
        state.active_profile = Some(profile("External"));
        state.profile_path = Some("/external/External.toml".into());
        state.active_profile_origin = Some(ProfileOrigin::External);
        drop(state);
        c.refresh(o);
        let mut meta = inputforge_app::bridge::prepare(&s.read(), None).meta;
        meta.profile_rows
            .iter_mut()
            .find(|r| r.is_active && !r.can_open)
            .unwrap()
            .last_edited_label = Some("7m ago".into());
        c.bindings().reconcile.call(meta.clone());
        assert_eq!(
            v.peek().rows[0].row.last_edited_label.as_deref(),
            Some("7m ago")
        );
        let _ = c.dispatch(o, Intent::Filter("missing".into()));
        assert_eq!(
            v.peek().rows[0].row.last_edited_label.as_deref(),
            Some("7m ago")
        );
        s.write().active_profile = Some(profile("External"));
        c.bindings().reconcile.call(meta);
        assert!(v.peek().rows[0].row.last_edited_label.is_none());
    });
}

#[test]
fn unavailable_library_metadata_clears_prior_label() {
    check(true, |c, o, v, s, _| {
        s.write().profile_library_rows[0].last_edited_at = chrono::DateTime::from_timestamp(0, 0);
        c.refresh(o);
        assert!(v.peek().rows[0].row.last_edited_label.is_some());
        s.write().profile_library_rows[0].last_edited_at = None;
        c.refresh(o);
        assert!(v.peek().rows[0].row.last_edited_label.is_none());
    });
}
