//! Add/duplicate preparation, current collision checks and explicit commit.
use super::super::Controller;
use inputforge_core::action::Mapping;
use inputforge_gui_dx::mapping_rail::{AddIntent, AddRequest, AddView, Phase, Scope, Target};
impl Controller {
    /// Prepare the next Add pad state and deliver only an explicit commit.
    #[must_use]
    pub fn add(&self, request: AddRequest) -> AddView {
        let AddRequest { mut view, intent } = request;
        if intent == AddIntent::CaptureCancelled && view.phase == Phase::Resting {
            return view;
        }
        if matches!(intent, AddIntent::Cancel | AddIntent::CaptureCancelled) {
            return AddView::default();
        }
        if let AddIntent::Start { scope, source } = intent {
            return self.start_add(scope, source);
        }
        let Some(scope) = view.scope.clone() else {
            return view;
        };
        let (source, collision) = {
            let state = self.state.read();
            if !self.valid_scope(&state, &scope) {
                return invalid();
            }
            self.inner
                .borrow_mut()
                .history
                .reconcile(&state, self.owner);
            let source = match &view.source {
                Some(before) => match self.current_target(
                    &state,
                    &Target {
                        scope: scope.clone(),
                        before: before.clone(),
                    },
                ) {
                    Some(current) => Some(current),
                    None => return invalid(),
                },
                None => None,
            };
            let input = match &intent {
                AddIntent::Captured(input) => Some(input),
                _ => match &view.phase {
                    Phase::Captured(input) => Some(input),
                    Phase::Collision(m) => Some(&m.input),
                    _ => None,
                },
            };
            let collision = input.and_then(|input| {
                self.inner
                    .borrow()
                    .history
                    .current(&state, &(scope.mode.clone(), input.clone()))
            });
            (source, collision)
        };
        let clear_error = intent != AddIntent::Reconcile;
        match intent {
            AddIntent::Captured(input) => {
                view.phase = collision.map_or(Phase::Captured(input), Phase::Collision);
            }
            AddIntent::Refresh => view.phase = Phase::Capturing,
            AddIntent::Reconcile => {
                if let Phase::Collision(before) = &view.phase {
                    view.phase =
                        collision.map_or(Phase::Captured(before.input.clone()), Phase::Collision);
                }
            }
            AddIntent::Open => {
                if let Phase::Collision(before) = &view.phase
                    && collision.as_ref() == Some(before)
                {
                    self.navigate(&(scope.mode, before.input.clone()));
                    return AddView::default();
                }
                return invalid();
            }
            AddIntent::Commit(name) => {
                let Phase::Captured(_) = &view.phase else {
                    return view;
                };
                if let Some(existing) = collision {
                    view.phase = Phase::Collision(existing);
                    return view;
                }
                return self.commit_add(view, &name, source);
            }
            AddIntent::Start { .. } | AddIntent::Cancel | AddIntent::CaptureCancelled => {
                unreachable!("handled before target validation")
            }
        }
        if clear_error {
            view.error = None;
        }
        view
    }
    fn start_add(&self, scope: Scope, source: Option<Target>) -> AddView {
        let state = self.state.read();
        if !self.valid_scope(&state, &scope) {
            return invalid();
        }
        let source = match source {
            Some(target) if target.scope == scope => match self.current_target(&state, &target) {
                Some(current) => Some(current),
                None => return invalid(),
            },
            Some(_) => return invalid(),
            None => None,
        };
        AddView {
            phase: Phase::Capturing,
            scope: Some(scope),
            source,
            error: None,
        }
    }
    fn commit_add(&self, mut view: AddView, name: &str, source: Option<Mapping>) -> AddView {
        let Some(scope) = view.scope.clone() else {
            return invalid();
        };
        let Phase::Captured(input) = &view.phase else {
            return view;
        };
        let name = name.trim();
        let mapping = Mapping {
            input: input.clone(),
            mode: scope.mode.clone(),
            name: (!name.is_empty()).then(|| name.to_owned()),
            actions: source.map_or_else(Vec::new, |m| m.actions),
        };
        if inputforge_core::profile::validate_mapping_action_tree(&mapping.input, &mapping.actions)
            .is_err()
        {
            view.error = Some("These actions cannot be copied to that input.".into());
            return view;
        }
        let key = (scope.mode, input.clone());
        if !self.forward(&key, Some(&mapping)) {
            view.error = Some("Mapping command could not be delivered. Try again.".into());
            return view;
        }
        self.navigate(&key);
        AddView::default()
    }
}
fn invalid() -> AddView {
    AddView {
        error: Some("This mapping changed or is no longer available. Start again.".into()),
        ..AddView::default()
    }
}
