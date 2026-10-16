//! Prepare mode operations with shared core validation.
use super::dispatch::STALE;
use inputforge_core::{
    engine::{EngineCommand, validate_mode_deletion, validate_mode_name},
    profile::Profile,
};
use inputforge_gui_dx::mode_management::{Operation, Request, Target, Transition, View};
pub(super) fn prepare(
    profile: &Profile,
    view: &View,
    request: &Request,
) -> Result<(Option<EngineCommand>, Transition), String> {
    let rows = &view.rows;
    let row = |name: &str| {
        rows.iter()
            .find(|r| r.name == name)
            .ok_or_else(|| STALE.to_owned())
    };
    let (command, transition) = match request.operation.clone() {
        Operation::BeginAdd => return Ok((None, Transition::Add)),
        Operation::BeginRename(name) => {
            row(&name)?;
            return Ok((None, Transition::Rename(name)));
        }
        Operation::BeginDelete(name) => {
            let r = row(&name)?;
            validate_mode_deletion(profile, &name).map_err(|e| e.to_string())?;
            return Ok((
                None,
                Transition::Delete(Target {
                    scope: request.scope.clone(),
                    name,
                    mappings: r.mappings,
                }),
            ));
        }
        Operation::Cancel(_) | Operation::CloseMenu { .. } => {
            unreachable!("dismissal handled before validation")
        }
        Operation::Select(name) => {
            row(&name)?;
            return Ok((None, Transition::Close(None)));
        }
        Operation::Add { raw, blur } => {
            let Some(name) = name(&raw, blur)? else {
                return Ok((None, Transition::Close(None)));
            };
            profile
                .modes()
                .with_appended(&name)
                .map_err(|e| e.to_string())?;
            (
                EngineCommand::AddMode { name: name.clone() },
                Transition::Close(Some(name)),
            )
        }
        Operation::Rename { from, raw, blur } => {
            row(&from)?;
            validate_mode_name(&from, "source mode name cannot be empty")
                .map_err(|e| e.to_string())?;
            let Some(to) = name(&raw, blur)? else {
                return Ok((None, Transition::Close(None)));
            };
            profile
                .modes()
                .with_renamed(&from, &to)
                .map_err(|e| e.to_string())?;
            if from == to {
                return Ok((None, Transition::Close(None)));
            }
            let focus = (request.scope.editing == from).then_some(to.clone());
            (
                EngineCommand::RenameMode { from, to },
                Transition::Close(focus),
            )
        }
        Operation::Delete(target) => {
            let count = row(&target.name)?.mappings;
            deletion(profile, request, target, count)?
        }
        Operation::Activate(name) => {
            if !row(&name)?.can_activate {
                return Ok((None, Transition::Close(Some(name))));
            }
            (
                EngineCommand::SwitchMode { mode: name.clone() },
                Transition::Close(Some(name)),
            )
        }
        Operation::Default(name) => {
            row(&name)?;
            validate_mode_name(&name, "startup mode name cannot be empty")
                .map_err(|e| e.to_string())?;
            if profile.settings().startup_mode() == name {
                return Ok((None, Transition::Close(Some(name))));
            }
            (
                EngineCommand::SetDefaultMode { name: name.clone() },
                Transition::Close(Some(name)),
            )
        }
    };
    Ok((Some(command), transition))
}
fn name(raw: &str, blur: bool) -> Result<Option<String>, String> {
    let name = raw.trim();
    if blur && name.is_empty() {
        return Ok(None);
    }
    validate_mode_name(name, "Name cannot be empty").map_err(|e| e.to_string())?;
    Ok(Some(name.to_owned()))
}

fn deletion(
    profile: &Profile,
    request: &Request,
    target: Target,
    count: usize,
) -> Result<(EngineCommand, Transition), String> {
    if target.scope != request.scope || count != target.mappings {
        return Err(
            "The affected mappings changed. Review the counts and reopen confirmation.".into(),
        );
    }
    let survivors = validate_mode_deletion(profile, &target.name).map_err(|e| e.to_string())?;
    let index = profile
        .modes()
        .as_slice()
        .iter()
        .position(|n| n == &target.name)
        .ok_or(STALE)?;
    let focus = survivors
        .as_slice()
        .get(index.min(survivors.len() - 1))
        .cloned();
    Ok((
        EngineCommand::DeleteMode { name: target.name },
        Transition::Close(focus),
    ))
}
