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

use std::path::PathBuf;

use dioxus::prelude::*;
use state::SheetsState;

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

        sheets.write().mark_saving();
        sheets.read().apply_to_documents(&mut next_documents);

        match authoring::save_sheets_documents(&mut next_documents) {
            Ok(()) => {
                documents.set(Some(next_documents));
                sheets.write().mark_saved();
            }
            Err(err) => sheets.write().mark_failed(err.to_string()),
        }
    });

    let retry_save = use_callback(move |()| {
        save_now.call(());
    });

    let import_asset = use_callback(move |source_path: PathBuf| {
        let previous_autosave = sheets.read().autosave;
        let Some(mut next_documents) = documents.read().clone() else {
            sheets
                .write()
                .mark_failed("sheet documents are not loaded".to_owned());
            return;
        };

        sheets.read().apply_to_documents(&mut next_documents);

        match authoring::import_template_asset_into(&source_path, &mut next_documents) {
            Ok(_imported) => {
                let mut next_state = SheetsState::from_documents(&next_documents);
                next_state.autosave = previous_autosave;
                documents.set(Some(next_documents));
                sheets.set(next_state);
            }
            Err(err) => sheets.write().mark_failed(err.to_string()),
        }
    });

    use_context_provider(|| SheetsWorkbenchActions {
        save_now,
        retry_save,
        import_asset,
    });

    let documents_loaded = documents.read().is_some();

    rsx! {
        div {
            class: "if-sheets",
            "data-testid": "sheets-workbench",
            "data-documents-loaded": documents_loaded,
            left_rail::SheetsLeftRail { sheets }
            canvas::SheetsCanvas { sheets }
            inspector::SheetsInspector { sheets }
        }
    }
}
