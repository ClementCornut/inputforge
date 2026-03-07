// Rust guideline compliant 2026-03-07

//! Floating profile management window.
//!
//! Lists discovered profiles, supports creating, renaming, loading, and
//! deleting profiles. Uses a `needs_refresh` guard so `list_profiles()`
//! is never called in the per-frame render path.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use inputforge_core::engine::EngineCommand;
use inputforge_core::profile::manager::{
    ProfileSummary, create_profile, delete_profile, list_profiles, rename_profile,
    validate_profile_name,
};
use inputforge_core::settings::AppSettings;

use crate::theme;
use crate::widgets::empty_state;
use crate::widgets::toast::ToastLevel;

/// Default window width in logical pixels.
const DEFAULT_WIDTH: f32 = 380.0;

/// Default window height in logical pixels.
const DEFAULT_HEIGHT: f32 = 420.0;

/// Minimum window width in logical pixels.
const MIN_WIDTH: f32 = 320.0;

/// Minimum window height in logical pixels.
const MIN_HEIGHT: f32 = 300.0;

/// Viewport ID for the native profile management window.
#[must_use]
pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("profile_window")
}

/// Persistent state for the profile management window.
#[derive(Debug, Default)]
pub(crate) struct ProfileWindowState {
    /// Cached profile summaries (sole data source for rendering).
    profiles: Vec<ProfileSummary>,
    /// Set to true to trigger a refresh from disk.
    needs_refresh: bool,
    /// Currently selected profile path.
    selected_path: Option<PathBuf>,
    /// Active inline rename: (path being renamed, current text).
    renaming: Option<(PathBuf, String)>,
    /// Path of profile pending delete confirmation.
    delete_confirming: Option<PathBuf>,
    /// Path of the profile row hovered last frame (for hover-reveal icons).
    hovered_path: Option<PathBuf>,
}

impl ProfileWindowState {
    /// Create a new state that will load profiles on first frame.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            needs_refresh: true,
            ..Self::default()
        }
    }
}

/// Show the profile management window as a native OS window.
///
/// Returns a list of toast messages to be pushed by the caller, avoiding
/// borrow conflicts with `ToastManager` on `InputForgeApp`.
pub(crate) fn show(
    ctx: &egui::Context,
    window_state: &mut ProfileWindowState,
    open: &mut bool,
    active_profile_path: Option<&Path>,
    commands: &mpsc::Sender<EngineCommand>,
    settings: &mut AppSettings,
) -> Vec<(String, ToastLevel)> {
    let mut toasts: Vec<(String, ToastLevel)> = Vec::new();

    if !*open {
        return toasts;
    }

    ctx.show_viewport_immediate(
        viewport_id(),
        egui::ViewportBuilder::default()
            .with_title("InputForge \u{2014} Profiles")
            .with_inner_size([DEFAULT_WIDTH, DEFAULT_HEIGHT])
            .with_min_inner_size([MIN_WIDTH, MIN_HEIGHT]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                *open = false;
            }

            // Refresh profile list from disk when flagged.
            if window_state.needs_refresh {
                match list_profiles() {
                    Ok(profiles) => window_state.profiles = profiles,
                    Err(e) => {
                        toasts.push((format!("Failed to list profiles: {e}"), ToastLevel::Error));
                        window_state.profiles = Vec::new();
                    }
                }
                window_state.needs_refresh = false;
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                let colors = theme::colors(ui.ctx());

                // --- Header row: title + "+ New" button ---
                show_header(ui, window_state, &mut toasts);

                ui.separator();

                // --- Profile list (scrollable) ---
                if window_state.profiles.is_empty() {
                    empty_state::empty_state(ui, "No profiles \u{2014} click + New to create one");
                } else {
                    let outcomes = show_profile_list(ui, window_state, active_profile_path, colors);

                    if let Some(outcome) = outcomes.rename {
                        apply_rename(window_state, &outcome, settings, commands, &mut toasts);
                    }
                    if let Some(outcome) = outcomes.delete {
                        apply_delete(window_state, &outcome, settings, &mut toasts);
                    }
                    if let Some(path) = outcomes.load_path {
                        apply_load(
                            &path,
                            commands,
                            settings,
                            &mut toasts,
                            &window_state.profiles,
                        );
                    }
                }
            });
        },
    );

    toasts
}

/// Render the header row with title and "+ New" button.
fn show_header(
    ui: &mut egui::Ui,
    window_state: &mut ProfileWindowState,
    toasts: &mut Vec<(String, ToastLevel)>,
) {
    ui.horizontal(|ui| {
        ui.heading("Profiles");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ New").clicked() {
                match create_profile("New Profile") {
                    Ok(new_path) => {
                        window_state.needs_refresh = true;
                        window_state.selected_path = Some(new_path.clone());
                        // Enter rename mode so the user can name it immediately.
                        window_state.renaming = Some((new_path, "New Profile".to_owned()));
                    }
                    Err(e) => {
                        toasts.push((format!("Failed to create profile: {e}"), ToastLevel::Error));
                    }
                }
            }
        });
    });
}

/// Deferred outcomes collected during profile list rendering.
struct ListOutcomes {
    rename: Option<RenameOutcome>,
    delete: Option<DeleteOutcome>,
    load_path: Option<PathBuf>,
}

/// Render the scrollable profile list and return deferred outcomes.
fn show_profile_list(
    ui: &mut egui::Ui,
    window_state: &mut ProfileWindowState,
    active_profile_path: Option<&Path>,
    colors: &theme::ThemeColors,
) -> ListOutcomes {
    // Collect results from rename/delete operations to apply after iteration.
    let mut rename_result: Option<RenameOutcome> = None;
    let mut delete_result: Option<DeleteOutcome> = None;
    let mut load_path: Option<PathBuf> = None;
    let mut new_hovered_path: Option<PathBuf> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Clone paths to avoid borrowing window_state during iteration.
            let profiles: Vec<(String, PathBuf)> = window_state
                .profiles
                .iter()
                .map(|p| (p.name.clone(), p.path.clone()))
                .collect();

            for (name, path) in &profiles {
                let is_selected = window_state.selected_path.as_ref() == Some(path);
                let is_active = active_profile_path == Some(path.as_path());
                let is_delete_confirming = window_state.delete_confirming.as_ref() == Some(path);

                // --- Delete confirmation mode ---
                if is_delete_confirming {
                    ui.horizontal(|ui| {
                        ui.label(format!("Delete {name}?"));
                        let delete_btn =
                            egui::Button::new(egui::RichText::new("Delete").color(colors.text))
                                .fill(colors.error);
                        if ui.add(delete_btn).clicked() {
                            delete_result = Some(DeleteOutcome {
                                path: path.clone(),
                                was_active: is_active,
                                name: name.clone(),
                            });
                        }
                        if ui.button("Cancel").clicked() {
                            window_state.delete_confirming = None;
                        }
                    });
                    continue;
                }

                // --- Rename mode ---
                let is_renaming = window_state
                    .renaming
                    .as_ref()
                    .is_some_and(|(rp, _)| rp == path);

                if is_renaming {
                    if let Some(outcome) = show_rename_row(ui, window_state, is_active) {
                        rename_result = Some(outcome);
                    }
                    continue;
                }

                // --- Normal display mode ---
                let is_hovered = window_state.hovered_path.as_ref() == Some(path);

                let response = ui.horizontal(|ui| {
                    let label = ui.selectable_label(is_selected, name);
                    if label.clicked() {
                        window_state.selected_path = Some(path.clone());
                    }
                    if label.double_clicked() {
                        load_path = Some(path.clone());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_active {
                            ui.colored_label(colors.live, "ACTIVE");
                        }

                        // Hover-reveal action icons (based on previous frame's hover).
                        if is_hovered {
                            match show_hover_icons(ui) {
                                Some(HoverAction::Load) => {
                                    load_path = Some(path.clone());
                                }
                                Some(HoverAction::Rename) => {
                                    window_state.renaming = Some((path.clone(), name.clone()));
                                }
                                Some(HoverAction::Delete) => {
                                    window_state.delete_confirming = Some(path.clone());
                                }
                                None => {}
                            }
                        }
                    });
                });

                // Track hover for next frame.
                if response.response.contains_pointer() {
                    new_hovered_path = Some(path.clone());
                }

                // Also select on row click.
                if response.response.interact(egui::Sense::click()).clicked() {
                    window_state.selected_path = Some(path.clone());
                }
            }
        });

    // Update hover tracking for next frame.
    window_state.hovered_path = new_hovered_path;

    ListOutcomes {
        rename: rename_result,
        delete: delete_result,
        load_path,
    }
}

/// Render inline rename text field. Returns a `RenameOutcome` when the user
/// commits the name (Enter or focus loss), or `None` if still editing / cancelled.
fn show_rename_row(
    ui: &mut egui::Ui,
    window_state: &mut ProfileWindowState,
    is_active: bool,
) -> Option<RenameOutcome> {
    let (rename_path, rename_text) = window_state.renaming.as_mut().expect("caller checked");

    let text_edit =
        egui::TextEdit::singleline(rename_text).desired_width(ui.available_width() - 8.0);
    let response = ui.add(text_edit);

    // Auto-focus only when the field doesn't already have focus.
    if !response.has_focus() && !response.lost_focus() {
        response.request_focus();
    }

    // Cancel on Escape.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        window_state.renaming = None;
        return None;
    }

    // Commit on Enter or loss of focus (but not Escape — handled above).
    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
    if enter_pressed || (response.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape))) {
        let new_name = rename_text.trim().to_owned();
        return Some(RenameOutcome {
            old_path: rename_path.clone(),
            new_name,
            was_active: is_active,
        });
    }

    None
}

/// Action triggered by a hover-reveal icon button.
enum HoverAction {
    Load,
    Rename,
    Delete,
}

/// Render hover-reveal icon buttons (Load, Rename, Delete) and return the
/// clicked action, if any.
fn show_hover_icons(ui: &mut egui::Ui) -> Option<HoverAction> {
    let mut action = None;

    let delete_btn = egui::Button::new(egui::RichText::new("\u{1F5D1}").small()).frame(false);
    if ui.add(delete_btn).on_hover_text("Delete").clicked() {
        action = Some(HoverAction::Delete);
    }

    let rename_btn = egui::Button::new(egui::RichText::new("\u{270F}").small()).frame(false);
    if ui.add(rename_btn).on_hover_text("Rename").clicked() {
        action = Some(HoverAction::Rename);
    }

    let load_btn = egui::Button::new(egui::RichText::new("\u{25B6}").small()).frame(false);
    if ui.add(load_btn).on_hover_text("Load").clicked() {
        action = Some(HoverAction::Load);
    }

    action
}

// ---------------------------------------------------------------------------
// Action outcomes
// ---------------------------------------------------------------------------

/// Deferred result from a rename operation.
struct RenameOutcome {
    old_path: PathBuf,
    new_name: String,
    was_active: bool,
}

/// Deferred result from a delete operation.
struct DeleteOutcome {
    path: PathBuf,
    was_active: bool,
    name: String,
}

// ---------------------------------------------------------------------------
// Action helpers
// ---------------------------------------------------------------------------

/// Apply a rename operation, updating state and settings as needed.
fn apply_rename(
    window_state: &mut ProfileWindowState,
    outcome: &RenameOutcome,
    settings: &mut AppSettings,
    commands: &mpsc::Sender<EngineCommand>,
    toasts: &mut Vec<(String, ToastLevel)>,
) {
    window_state.renaming = None;

    if let Err(e) = validate_profile_name(&outcome.new_name) {
        toasts.push((format!("Invalid name: {e}"), ToastLevel::Error));
        return;
    }

    match rename_profile(&outcome.old_path, &outcome.new_name) {
        Ok(new_path) => {
            if outcome.was_active {
                settings.last_profile = Some(new_path.clone());
                if let Err(e) = settings.save() {
                    tracing::warn!(error = %e, "failed to save settings after rename");
                }
                if let Err(e) = commands.send(EngineCommand::LoadProfile(new_path.clone())) {
                    tracing::warn!(error = %e, "failed to send LoadProfile after rename");
                }
            }
            window_state.selected_path = Some(new_path);
            window_state.needs_refresh = true;
        }
        Err(e) => {
            toasts.push((format!("Rename failed: {e}"), ToastLevel::Error));
        }
    }
}

/// Apply a delete operation, updating state as needed.
fn apply_delete(
    window_state: &mut ProfileWindowState,
    outcome: &DeleteOutcome,
    settings: &mut AppSettings,
    toasts: &mut Vec<(String, ToastLevel)>,
) {
    window_state.delete_confirming = None;

    match delete_profile(&outcome.path) {
        Ok(()) => {
            if outcome.was_active {
                settings.last_profile = None;
                let _ = settings.save();
                toasts.push((
                    format!(
                        "Active profile '{}' deleted \u{2014} engine still running with in-memory copy",
                        outcome.name
                    ),
                    ToastLevel::Warning,
                ));
            }
            if window_state.selected_path.as_ref() == Some(&outcome.path) {
                window_state.selected_path = None;
            }
            window_state.needs_refresh = true;
        }
        Err(e) => {
            toasts.push((format!("Delete failed: {e}"), ToastLevel::Error));
        }
    }
}

/// Load a profile by path, updating settings and sending the engine command.
fn apply_load(
    path: &Path,
    commands: &mpsc::Sender<EngineCommand>,
    settings: &mut AppSettings,
    toasts: &mut Vec<(String, ToastLevel)>,
    profiles: &[ProfileSummary],
) {
    let name = profiles
        .iter()
        .find(|p| p.path == *path)
        .map_or("unknown", |p| p.name.as_str());

    if let Err(e) = commands.send(EngineCommand::LoadProfile(path.to_path_buf())) {
        tracing::warn!(error = %e, "failed to send LoadProfile command");
    }

    settings.last_profile = Some(path.to_path_buf());
    if let Err(e) = settings.save() {
        tracing::warn!(error = %e, "failed to save settings after load");
    }

    toasts.push((format!("Loaded '{name}'"), ToastLevel::Info));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_window_state_default_needs_no_refresh() {
        let state = ProfileWindowState::default();
        assert!(!state.needs_refresh);
        assert!(state.profiles.is_empty());
        assert!(state.selected_path.is_none());
        assert!(state.renaming.is_none());
        assert!(state.delete_confirming.is_none());
        assert!(state.hovered_path.is_none());
    }

    #[test]
    fn profile_window_state_new_needs_refresh() {
        let state = ProfileWindowState::new();
        assert!(state.needs_refresh);
    }

    const _: () = assert!(DEFAULT_WIDTH > 0.0);
    const _: () = assert!(DEFAULT_HEIGHT > 0.0);
    const _: () = assert!(MIN_WIDTH > 0.0);
    const _: () = assert!(MIN_HEIGHT > 0.0);
}
