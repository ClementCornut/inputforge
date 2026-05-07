#![cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into profile creation UI in later tasks")
)]

use std::path::PathBuf;

use inputforge_core::engine::EngineCommand;

use crate::frame::profiles::actions::{NewProfileValidationError, validate_new_profile_name};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    test,
    expect(
        dead_code,
        reason = "copy/open profile variants are covered by later UI sub-modes"
    )
)]
pub(crate) enum NewProfileSource {
    Blank,
    CopyActive,
    CopyProfile(PathBuf),
    OpenPath(PathBuf),
}

pub(crate) fn create_new_profile_command(
    source: NewProfileSource,
    name: &str,
    active_path: Option<PathBuf>,
    existing_names: &[String],
) -> Result<EngineCommand, NewProfileValidationError> {
    let name = validate_new_profile_name(name, existing_names)?;
    match source {
        NewProfileSource::Blank => Ok(EngineCommand::CreateProfile { name }),
        NewProfileSource::CopyActive => {
            // Missing active path is treated as MissingPath, since the
            // user must pick a source profile before Create can fire.
            let source_path = active_path.ok_or(NewProfileValidationError::MissingPath)?;
            Ok(EngineCommand::DuplicateProfile { source_path, name })
        }
        NewProfileSource::CopyProfile(source_path) => {
            Ok(EngineCommand::DuplicateProfile { source_path, name })
        }
        NewProfileSource::OpenPath(path) => {
            Ok(EngineCommand::AddExternalProfileToLibrary { path, name })
        }
    }
}

pub(crate) fn open_file_load_once_command(
    path: PathBuf,
) -> Result<EngineCommand, NewProfileValidationError> {
    if path.as_os_str().is_empty() {
        return Err(NewProfileValidationError::MissingPath);
    }
    Ok(EngineCommand::LoadExternalProfileOnce(path))
}

pub(crate) fn add_external_to_library_command(
    path: PathBuf,
    name: &str,
    existing_names: &[String],
) -> Result<EngineCommand, NewProfileValidationError> {
    if path.as_os_str().is_empty() {
        return Err(NewProfileValidationError::MissingPath);
    }
    Ok(EngineCommand::AddExternalProfileToLibrary {
        path,
        name: validate_new_profile_name(name, existing_names)?,
    })
}
