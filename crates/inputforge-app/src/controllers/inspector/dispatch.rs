//! Captured inspector observations resolve against authoritative state before delivery.
use super::{Controller, Session, policy};
use dioxus::prelude::*;
use inputforge_core::engine::{DeviceAction, DeviceRequest, DeviceTarget, EngineCommand};
use inputforge_core::{state::AppState, types::DeviceId};
use inputforge_gui_dx::controllers::inspector::{Intent, View};
impl Controller {
    pub(super) fn dispatch(&self, owner: u64, workflow: u64, intent: Intent) -> bool {
        self.dispatch_with(owner, workflow, intent, |report| {
            self.clipboard
                .borrow_mut()
                .write(report)
                .map_err(|e| e.to_string())
        })
    }
    pub(super) fn dispatch_with(
        &self,
        owner: u64,
        workflow: u64,
        intent: Intent,
        copy: impl FnOnce(&str) -> Result<(), String>,
    ) -> bool {
        let mut sessions = self.session.borrow_mut();
        let Some(s) = sessions
            .as_mut()
            .filter(|s| s.owner == owner && s.view.peek().workflow == workflow)
        else {
            return false;
        };
        let Some(state) = self.state.try_read() else {
            s.view.write().error = Some("Device settings are busy. Try again.".into());
            return false;
        };
        let target = s.target.clone();
        if !matches!(&intent, Intent::Select(_))
            && target.as_ref().is_none_or(|t| !t.matches(&state))
        {
            s.view.write().error =
                Some("Device or profile changed. Reselect the device and try again.".into());
            return false;
        }
        let mut next = s.view.peek().clone();
        let result = (|| {
            if let Intent::Select(id) = intent {
                select(s, &mut next, &state, &id)?;
            } else {
                let rows = crate::bridge::devices::build_device_panel_rows(&state);
                let current = rows
                    .iter()
                    .find(|r| Some(&r.device_id) == target.as_ref().map(|t| &t.info.id))
                    .cloned()
                    .ok_or("Device is no longer available.")?;
                next.rows = rows;
                next.selected = Some(current.clone());
                let action = match intent {
                    Intent::Input(value) => {
                        next.draft = value;
                        None
                    }
                    Intent::Cancel => {
                        next.draft = current.alias;
                        None
                    }
                    Intent::Save => {
                        let alias = next.draft.trim();
                        (alias != current.alias).then(|| {
                            DeviceAction::Alias((!alias.is_empty()).then(|| alias.to_owned()))
                        })
                    }
                    Intent::Remove => {
                        policy::availability(&mut next, &state);
                        if let Some(reason) = &next.removal_reason {
                            return Err(reason.clone());
                        }
                        Some(DeviceAction::Remove)
                    }
                    Intent::Copy(report) => {
                        drop(state);
                        copy(&report).map_err(|e| format!("Copy failed: {e}"))?;
                        return Ok(None);
                    }
                    Intent::Select(_) => unreachable!("selection handled above"),
                };
                policy::availability(&mut next, &state);
                drop(state);
                return Ok(action);
            }
            policy::availability(&mut next, &state);
            Ok(None)
        })();
        let result = result.and_then(|action| {
            if let Some(action) = action {
                self.commands
                    .send(EngineCommand::Device(Box::new(DeviceRequest {
                        target: target.expect("selected target validated"),
                        action,
                    })))
                    .map_err(|_e| "Could not send device change. Try again.".to_owned())?;
            }
            Ok(())
        });
        match result {
            Ok(()) => {
                next.error = None;
                s.view.set(next);
                true
            }
            Err(error) => {
                s.view.write().error = Some(error);
                false
            }
        }
    }
}

fn select(s: &mut Session, next: &mut View, state: &AppState, id: &DeviceId) -> Result<(), String> {
    if !s
        .catalog
        .iter()
        .any(|t| &t.info.id == id && t.matches(state))
    {
        return Err("Device or profile changed. Reselect the device and try again.".into());
    }
    let rows = crate::bridge::devices::build_device_panel_rows(state);
    let row = rows
        .iter()
        .find(|r| &r.device_id == id)
        .cloned()
        .ok_or("Device is no longer available.")?;
    s.target = DeviceTarget::capture(state, id);
    next.rows = rows;
    next.draft.clone_from(&row.alias);
    next.selected = Some(row);
    next.workflow += 1;

    Ok(())
}
