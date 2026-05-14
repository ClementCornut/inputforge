use dioxus::prelude::*;

use crate::frame::sheets::state::{AutosaveStatus, SheetsState};

#[component]
pub(crate) fn SheetsInspector(sheets: Signal<SheetsState>) -> Element {
    let save_status = match sheets.read().autosave {
        AutosaveStatus::Clean => "Saved",
        AutosaveStatus::Dirty => "Unsaved",
        AutosaveStatus::Saving => "Saving",
        AutosaveStatus::Failed => "Save failed",
    };

    rsx! {
        aside { "data-testid": "sheets-inspector",
            h2 { "Inspector" }
            p { "{save_status}" }
        }
    }
}
