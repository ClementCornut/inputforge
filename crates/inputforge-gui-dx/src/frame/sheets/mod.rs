#![allow(
    dead_code,
    reason = "Task 3 wires the Sheets authoring shell ahead of later navigation, import, autosave, and styling tasks."
)]

mod authoring;
mod canvas;
mod inspector;
mod left_rail;
pub(crate) mod state;
#[cfg(test)]
mod tests;

use std::{path::PathBuf, time::Duration};

use dioxus::prelude::*;
use inputforge_core::sheet::AnchorAssignment;
use state::{AutosaveStatus, CaptureStatus, SheetsState};

use crate::patterns::live_capture::{CaptureFilter, LiveCapture, is_current_capture_session};

const SHEETS_CSS: Asset = asset!("/assets/frame/sheets.css");
// Short enough to feel live while coalescing rapid inspector and canvas edits.
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(400);

#[derive(Clone, Copy)]
pub(crate) struct SheetsWorkbenchActions {
    pub save_now: Callback<()>,
    pub retry_save: Callback<()>,
    pub import_asset: Callback<PathBuf>,
}

#[component]
pub(crate) fn SheetsWorkbench() -> Element {
    let mut sheets = use_signal(SheetsState::default);
    let mut documents = use_signal(|| Option::<authoring::SheetsDocuments>::None);
    let mut loaded = use_signal(|| false);
    let autosave_generation = use_signal(|| 0_u64);
    let capture = try_use_context::<LiveCapture>();
    let armed_capture_session: Signal<Option<u64>> = use_signal(|| None);

    use_effect(move || {
        if *loaded.read() {
            return;
        }

        loaded.set(true);
        match authoring::load_sheets_documents() {
            Ok(loaded_documents) => {
                sheets.set(SheetsState::from_documents(&loaded_documents));
                documents.set(Some(loaded_documents));
            }
            Err(err) => sheets.write().mark_failed(err.to_string()),
        }
    });

    let save_now = use_callback(move |()| {
        let Some(mut next_documents) = documents.read().clone() else {
            sheets
                .write()
                .mark_failed("sheet documents are not loaded".to_owned());
            return;
        };

        let current_state = sheets.read().clone();
        sheets.write().mark_saving();
        current_state.apply_to_documents(&mut next_documents);

        match authoring::save_sheets_documents(&mut next_documents) {
            Ok(()) => {
                documents.set(Some(next_documents));
                sheets.write().mark_saved();
            }
            Err(err) => sheets.write().mark_failed(err.to_string()),
        }
    });

    let mut autosave_generation_for_retry = autosave_generation;
    let retry_save = use_callback(move |()| {
        let generation = *autosave_generation_for_retry.peek() + 1;
        autosave_generation_for_retry.set(generation);
        save_now.call(());
    });

    let save_now_for_autosave = save_now;
    let mut autosave_generation_for_effect = autosave_generation;
    use_effect(move || {
        if !matches!(sheets.read().autosave, AutosaveStatus::Dirty) {
            return;
        }

        let generation = *autosave_generation_for_effect.peek() + 1;
        autosave_generation_for_effect.set(generation);
        let generation_for_save = autosave_generation_for_effect;
        let sheets_for_save = sheets;
        let save_now_for_save = save_now_for_autosave;

        spawn(async move {
            tokio::time::sleep(AUTOSAVE_DEBOUNCE).await;

            if *generation_for_save.read() != generation {
                return;
            }
            if !matches!(sheets_for_save.read().autosave, AutosaveStatus::Dirty) {
                return;
            }

            save_now_for_save.call(());
        });
    });

    let import_asset = use_callback(move |source_path: PathBuf| {
        let Some(mut next_documents) = documents.read().clone() else {
            sheets
                .write()
                .mark_failed("sheet documents are not loaded".to_owned());
            return;
        };
        let display_name = template_display_name_from_path(&source_path);

        sheets.read().apply_to_documents(&mut next_documents);

        match authoring::import_template_asset_into(&source_path, &mut next_documents) {
            Ok(imported) => {
                let mut next_state = sheets.read().clone();
                next_state.assets.clone_from(&next_documents.assets.assets);
                next_state
                    .asset_health
                    .clone_from(&next_documents.asset_health);
                next_state.create_template_from_asset(imported.entry.asset_id, display_name);
                next_state.apply_to_documents(&mut next_documents);
                documents.set(Some(next_documents));
                sheets.set(next_state);
            }
            Err(err) => sheets.write().mark_failed(err.to_string()),
        }
    });

    let on_import_image = use_callback(move |()| {
        let Some(source_path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
            .pick_file()
        else {
            return;
        };

        import_asset.call(source_path);
    });

    let capture_for_availability = capture;
    use_effect(move || {
        if capture_for_availability.is_some()
            && matches!(sheets.read().capture, CaptureStatus::Unavailable(_))
        {
            sheets.write().capture = CaptureStatus::Idle;
        }
    });

    let capture_for_arm = capture;
    let mut sheets_for_arm = sheets;
    let mut armed_capture_session_for_arm = armed_capture_session;
    let on_arm_capture = use_callback(move |anchor_id| {
        let Some(capture) = capture_for_arm else {
            sheets_for_arm.write().capture =
                CaptureStatus::Unavailable("live input is not available".to_owned());
            return;
        };

        capture.start.call(CaptureFilter::Any);
        armed_capture_session_for_arm.set(Some(*capture.session.peek()));
        sheets_for_arm.write().arm_capture(anchor_id);
    });

    let capture_for_cancel = capture;
    let mut sheets_for_cancel = sheets;
    let mut armed_capture_session_for_cancel = armed_capture_session;
    let on_cancel_capture = use_callback(move |()| {
        if let Some(capture) = capture_for_cancel {
            capture.cancel.call(());
        }
        armed_capture_session_for_cancel.set(None);
        sheets_for_cancel.write().cancel_capture();
    });

    let capture_for_external_cancel = capture;
    let mut sheets_for_external_cancel = sheets;
    let mut armed_capture_session_for_external_cancel = armed_capture_session;
    use_effect(move || {
        let Some(capture) = capture_for_external_cancel else {
            return;
        };
        if !should_clear_armed_capture_session(
            *armed_capture_session_for_external_cancel.peek(),
            *capture.active.read(),
            *capture.session.read(),
            capture.captured.read().is_some(),
        ) {
            return;
        }

        armed_capture_session_for_external_cancel.set(None);
        sheets_for_external_cancel.write().cancel_capture();
    });

    let capture_for_assignment = capture;
    let mut sheets_for_assignment = sheets;
    let mut armed_capture_session_for_assignment = armed_capture_session;
    use_effect(move || {
        let Some(capture) = capture_for_assignment else {
            return;
        };
        let captured_input = capture.captured.read().clone();
        if !is_current_capture_session(
            *armed_capture_session_for_assignment.peek(),
            *capture.session.peek(),
        ) {
            return;
        }
        let Some(input) = captured_input else {
            return;
        };

        let CaptureStatus::Armed(anchor_id) = sheets_for_assignment.read().capture.clone() else {
            armed_capture_session_for_assignment.set(None);
            return;
        };

        {
            let mut state = sheets_for_assignment.write();
            state.selected_anchor_id = Some(anchor_id.clone());
            state.assign_selected_anchor(input, AnchorAssignment::Captured);
            state.capture = CaptureStatus::Assigned(anchor_id);
        };
        armed_capture_session_for_assignment.set(None);
        let mut captured = capture.captured;
        captured.set(None);
    });

    use_context_provider(|| SheetsWorkbenchActions {
        save_now,
        retry_save,
        import_asset,
    });

    let documents_loaded = documents.read().is_some();

    rsx! {
        Stylesheet { href: SHEETS_CSS }
        div {
            class: "if-sheets",
            "data-testid": "sheets-workbench",
            "data-documents-loaded": documents_loaded,
            left_rail::SheetsLeftRail { sheets, on_import_image }
            canvas::SheetsCanvas { sheets, on_import_image, on_arm_capture }
            inspector::SheetsInspector {
                sheets,
                on_arm_capture,
                on_cancel_capture,
                on_retry_save: retry_save,
            }
        }
    }
}

fn template_display_name_from_path(source_path: &std::path::Path) -> String {
    source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Imported template")
        .to_owned()
}

fn should_clear_armed_capture_session(
    owned_session: Option<u64>,
    active_now: bool,
    current_session: u64,
    captured_input_pending: bool,
) -> bool {
    let owns_current_session = is_current_capture_session(owned_session, current_session);
    let captured_input_pending_for_owned_session = captured_input_pending && owns_current_session;

    owned_session.is_some()
        && !captured_input_pending_for_owned_session
        && (!active_now || !owns_current_session)
}

#[cfg(test)]
mod capture_session_tests {
    use super::*;

    #[test]
    fn watcher_clears_owned_capture_after_cancel_or_supersede_without_pending_input() {
        assert!(should_clear_armed_capture_session(Some(7), false, 7, false));
        assert!(should_clear_armed_capture_session(Some(7), true, 8, false));
        assert!(!should_clear_armed_capture_session(Some(7), false, 7, true));
        assert!(!should_clear_armed_capture_session(None, false, 7, false));
        assert!(!should_clear_armed_capture_session(Some(7), true, 7, false));
    }

    #[test]
    fn watcher_clears_superseded_owned_capture_with_pending_foreign_input() {
        assert!(should_clear_armed_capture_session(Some(7), true, 8, true));
    }
}
