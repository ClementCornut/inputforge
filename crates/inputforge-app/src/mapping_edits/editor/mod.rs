//! Translate editor intents using the effective queued mapping before shared dispatch.
mod defaults;
mod fields;
pub(super) mod predicate;
use super::{Controller, paths};
use inputforge_core::{action::Action, state::AppState};
use inputforge_gui_dx::mapping_edits::{Edit, Inline, Request, StageIdSegment};
use std::time::Instant;
impl Controller {
    pub(super) fn palette(
        &self,
        (key, path): (
            inputforge_gui_dx::snapshots::MappingKey,
            Vec<StageIdSegment>,
        ),
    ) -> inputforge_gui_dx::mapping_edits::Palette {
        use inputforge_core::action::ActionBranch;
        let state = self.state.read();
        inputforge_gui_dx::mapping_edits::Palette {
            gestures: key.1.is_button_shaped()
                && !path.iter().any(|segment| {
                    matches!(
                        segment,
                        StageIdSegment::Branch(
                            ActionBranch::TapSingle
                                | ActionBranch::TapDouble
                                | ActionBranch::PressShort
                                | ActionBranch::PressLong
                        )
                    )
                }),
            keyboard: state.session.keyboard_supported,
            mouse: state.session.mouse_supported,
        }
    }

    pub(super) fn inline(&self, mut intent: Inline) -> Result<bool, String> {
        if intent.cancel {
            return Ok(false);
        }
        if let Edit::Rename(raw) = &mut intent.edit {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(false);
            }
            *raw = trimmed.into();
        }
        let target = intent.target.ok_or_else(|| {
            "The mapping could not be updated. Reopen rename on the current mapping and try again.".to_owned()
        })?;
        if let Edit::Rebind(input) = &intent.edit
            && *input == target.before.input
        {
            return Ok(false);
        }
        self.edit(
            Request {
                profile: target.scope.profile,
                before: target.before,
                edit: intent.edit,
                label: intent.label,
                group: None,
                editor: intent.editor,
            },
            Instant::now(),
            true,
        )
    }
}
pub(super) fn resolve(
    edit: &Edit,
    actions: &[Action],
    state: &AppState,
    input: &inputforge_core::types::InputAddress,
) -> Option<Edit> {
    use inputforge_gui_dx::mapping_edits::{Field, StageKind};
    match edit {
        Edit::Add {
            kind: StageKind::Mouse,
            ..
        } if !state.session.mouse_supported => return None,
        Edit::Add {
            kind: StageKind::Keyboard(_),
            ..
        } if !state.session.keyboard_supported => return None,
        Edit::Field {
            field: Field::ModeHold(true),
            ..
        } if !input.is_button_shaped() => return None,
        Edit::Field {
            field: Field::ModeTarget(mode),
            ..
        } if !state.active_profile.as_ref()?.modes().contains(mode) => return None,
        _ => {}
    }
    Some(match edit {
        Edit::Field { path, field } => Edit::Replace {
            path: path.clone(),
            action: fields::replace(paths::at(actions, &path.0)?, field)?,
        },
        Edit::Add { path, kind } => Edit::Insert {
            path: path.clone(),
            action: defaults::action(kind, state),
        },
        Edit::Step { path, down } => {
            let mut to = path.clone();
            let StageIdSegment::Index(index) = to.0.last_mut()? else {
                return None;
            };
            *index = if *down {
                index.checked_add(1)?
            } else {
                index.checked_sub(1)?
            };
            paths::at(actions, &to.0)?;
            Edit::Move {
                from: path.clone(),
                to,
            }
        }
        _ => edit.clone(),
    })
}

pub(super) fn menu(
    actions: &[Action],
    path: &inputforge_gui_dx::mapping_edits::StageId,
) -> Option<inputforge_gui_dx::mapping_edits::StageMenu> {
    let (StageIdSegment::Index(index), parent) = path.0.split_last()? else {
        return None;
    };
    paths::at(actions, &path.0)?;
    let mut next = path.clone();
    *next.0.last_mut()? = StageIdSegment::Index(index.checked_add(1)?);
    Some(inputforge_gui_dx::mapping_edits::StageMenu {
        parent: parent.to_vec(),
        index: *index,
        up: *index > 0,
        down: paths::at(actions, &next.0).is_some(),
    })
}
