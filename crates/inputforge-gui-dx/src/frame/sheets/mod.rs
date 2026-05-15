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

const SHEETS_CSS: Asset = asset!("/assets/frame/sheets.css");

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
                next_state.assets = next_documents.assets.assets.clone();
                next_state.asset_health = next_documents.asset_health.clone();
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

    let on_arm_capture = use_callback(move |anchor_id| {
        sheets.write().arm_capture(anchor_id);
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
            inspector::SheetsInspector { sheets }
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
