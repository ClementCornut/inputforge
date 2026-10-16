use super::{Controller, Picker, Scope, dispatch, policy};
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, settings::AppSettings};
use inputforge_gui_dx::profiles::Mode;
use std::path::PathBuf;

impl Controller {
    /// Capture the current panel and profile before starting the OS picker.
    #[must_use]
    pub fn begin_pick(&self, owner: u64) -> Option<Picker> {
        let Some(state) = self.state.try_read() else {
            self.error(owner, "Profiles are busy. Try again.".into());
            return None;
        };
        let mut sessions = self.session.borrow_mut();
        let s = sessions.as_mut().filter(|s| s.owner == owner)?;
        s.picker += 1;
        Some(Picker {
            owner,
            serial: s.picker,
            scope: Scope::from_state(&state),
        })
    }
    pub(super) fn pick(&self, owner: u64) -> bool {
        let Some(request) = self.begin_pick(owner) else {
            return false;
        };
        let c = self.clone();
        spawn(async move {
            let path = rfd::AsyncFileDialog::new()
                .add_filter("Profile (TOML)", &["toml"])
                .set_title("Open profile")
                .pick_file()
                .await
                .map(|h| h.path().to_path_buf());
            c.finish_pick(&request, path);
        });
        true
    }
    /// Complete one native picker, preserving the form on cancellation or failure.
    pub fn finish_pick(&self, request: &Picker, path: Option<PathBuf>) {
        if !self
            .session
            .borrow()
            .as_ref()
            .is_some_and(|s| s.owner == request.owner && s.picker == request.serial)
        {
            return;
        }
        let Some(state) = self.state.try_read() else {
            self.error(request.owner, "Profiles are busy. Try again.".into());
            return;
        };
        let mut sessions = self.session.borrow_mut();
        let Some(s) = sessions.as_mut().filter(|s| {
            s.owner == request.owner
                && s.picker == request.serial
                && request.scope == Scope::from_state(&state)
        }) else {
            return;
        };
        s.picker += 1;
        let Some(path) = path else {
            return;
        };
        if path.as_os_str().is_empty() {
            return;
        }
        let mut view = s.view.peek().clone();
        let mut next = s.clone();
        let command = if path.starts_with(AppSettings::profiles_dir()) {
            dispatch::close(&mut next, &mut view);
            Some(EngineCommand::LoadProfile(path))
        } else {
            view.name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Imported")
                .into();
            view.workflow += 1;
            view.mode = Mode::Open { path };
            view.rename = None;
            view.delete = None;
            next.scope = Scope::from_state(&state);
            None
        };
        policy::project(&state, &mut view);
        drop(state);
        match command.map_or(Ok(()), |cmd| dispatch::send(&self.commands, cmd)) {
            Ok(()) => {
                view.error = None;
                *s = next;
                s.view.set(view);
            }
            Err(e) => s.view.write().error = Some(e),
        }
    }
}
