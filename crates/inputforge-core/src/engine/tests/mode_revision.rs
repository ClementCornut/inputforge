use super::*;
#[test]
fn mode_recreation_changes_revision_even_when_names_and_counts_match() {
    let (mut engine, state, _, _dir, _path) = make_engine_with_disk_profile();
    let original = state.read().mode_revision;
    let names = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .modes()
        .clone();
    engine
        .handle_command(EngineCommand::DeleteMode {
            name: "Landing".into(),
        })
        .unwrap();
    engine
        .handle_command(EngineCommand::AddMode {
            name: "Landing".into(),
        })
        .unwrap();
    assert_eq!(
        state.read().active_profile.as_ref().unwrap().modes(),
        &names
    );
    assert_eq!(state.read().mode_revision, original + 2);
    engine
        .handle_command(EngineCommand::RenameMode {
            from: "Landing".into(),
            to: "LANDING".into(),
        })
        .unwrap();
    assert_eq!(state.read().mode_revision, original + 3);
}
#[test]
fn failed_mode_save_still_invalidates_the_old_in_memory_generation() {
    let (mut engine, state, _, dir, _path) = make_engine_with_disk_profile();
    state.write().profile_path = Some(dir.path().to_path_buf());
    let original = state.read().mode_revision;
    engine
        .handle_command(EngineCommand::AddMode {
            name: "Approach".into(),
        })
        .unwrap_err();
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Approach")
    );
    assert_eq!(state.read().mode_revision, original + 1);
}
