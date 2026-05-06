#![cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into profile creation UI in later tasks")
)]

use std::path::PathBuf;

use inputforge_core::engine::EngineCommand;

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
) -> Result<EngineCommand, String> {
    let name = validate_name(name)?;
    match source {
        NewProfileSource::Blank => Ok(EngineCommand::CreateProfile { name }),
        NewProfileSource::CopyActive => {
            let source_path = active_path.ok_or_else(|| "no active profile to copy".to_owned())?;
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

pub(crate) fn open_file_load_once_command(path: PathBuf) -> EngineCommand {
    EngineCommand::LoadExternalProfileOnce(path)
}

pub(crate) fn add_external_to_library_command(
    path: PathBuf,
    name: &str,
) -> Result<EngineCommand, String> {
    Ok(EngineCommand::AddExternalProfileToLibrary {
        path,
        name: validate_name(name)?,
    })
}

fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("profile name cannot be empty".to_owned());
    }
    Ok(trimmed.to_owned())
}
