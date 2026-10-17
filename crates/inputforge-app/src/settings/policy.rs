//! Settings input policy delegates persisted validation to core.
use inputforge_core::{
    action::validate_gesture_threshold_ms,
    engine::{PruneGuard, SettingsChange},
    state::AppState,
};
use inputforge_gui_dx::{
    components::IntegerInputError,
    settings::{Intent, Prune, View},
};
pub(super) fn prepare(
    view: &mut View,
    state: &AppState,
    intent: Intent,
) -> Result<Option<SettingsChange>, String> {
    let mut guard = PruneGuard::capture(state);
    guard.config = view.values.snapshot.clone();
    let change = match intent {
        Intent::Max(text) => {
            let candidate = text
                .trim()
                .parse::<usize>()
                .map_err(|_error| "Must be a whole number between 1 and 100".to_owned())?;
            let mut config = view.values.snapshot.clone();
            config.max_count = candidate;
            config.validate().map_err(|e| e.to_string())?;
            if candidate == view.values.snapshot.max_count {
                return Ok(None);
            }
            let will_remove = if candidate < view.values.snapshot.max_count {
                guard.unpinned.saturating_sub(candidate)
            } else {
                0
            };
            if will_remove > 0 {
                view.workflow += 1;
                view.prune = Some(Prune {
                    candidate_max: candidate,
                    will_remove,
                    profile_name: state
                        .active_profile
                        .as_ref()
                        .map_or_else(|| "this profile".into(), |p| p.name().to_owned()),
                    guard,
                });
                return Ok(None);
            }
            view.values.snapshot.max_count = candidate;
            SettingsChange::Max {
                value: candidate,
                guard,
            }
        }
        Intent::Confirm => {
            let pending = view
                .prune
                .as_ref()
                .ok_or("No buffer change awaits confirmation.")?;
            if pending.guard != guard {
                return Err(
                    "Profile, snapshot count or settings changed. Cancel and try again.".into(),
                );
            }
            let value = pending.candidate_max;
            view.values.snapshot.max_count = value;
            let guard = pending.guard.clone();
            view.prune = None;
            view.workflow += 1;
            SettingsChange::Max { value, guard }
        }
        Intent::InvalidGesture(_error) => {
            return Err("Enter a whole number between 1 and 10000 ms".into());
        }
        Intent::Invalid(error) => {
            return Err(match error {
                IntegerInputError::Empty => "Enter a value between 1 and 100".into(),
                IntegerInputError::NotANumber => "Must be a whole number between 1 and 100".into(),
                IntegerInputError::OutOfRange { .. } => "Must be between 1 and 100".into(),
            });
        }
        Intent::Skip => {
            view.values.snapshot.skip_if_unchanged = !view.values.snapshot.skip_if_unchanged;
            SettingsChange::Skip(view.values.snapshot.skip_if_unchanged)
        }
        Intent::Autostart => {
            view.values.startup.launch_at_startup = !view.values.startup.launch_at_startup;
            SettingsChange::Autostart(view.values.startup.launch_at_startup)
        }
        Intent::Minimized => {
            view.values.startup.start_minimized_to_tray =
                !view.values.startup.start_minimized_to_tray;
            SettingsChange::Minimized(view.values.startup.start_minimized_to_tray)
        }
        Intent::DoubleTap(value) => {
            let value = u64::try_from(value).map_err(|e| e.to_string())?;
            validate_gesture_threshold_ms(value).map_err(|e| e.to_string())?;
            if value == view.values.default_double_tap_threshold_ms {
                return Ok(None);
            }
            view.values.default_double_tap_threshold_ms = value;
            SettingsChange::DoubleTap(value)
        }
        Intent::LongPress(value) => {
            let value = u64::try_from(value).map_err(|e| e.to_string())?;
            validate_gesture_threshold_ms(value).map_err(|e| e.to_string())?;
            if value == view.values.default_long_press_threshold_ms {
                return Ok(None);
            }
            view.values.default_long_press_threshold_ms = value;
            SettingsChange::LongPress(value)
        }
        Intent::Cancel => unreachable!("cancel handled before state acquisition"),
    };
    Ok(Some(change))
}
