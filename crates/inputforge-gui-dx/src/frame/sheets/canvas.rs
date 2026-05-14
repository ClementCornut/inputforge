use dioxus::prelude::*;

use crate::frame::sheets::state::SheetsState;

#[component]
pub(crate) fn SheetsCanvas(sheets: Signal<SheetsState>) -> Element {
    let has_template = sheets.read().selected_template_id.is_some();
    let label = if has_template {
        "Template image"
    } else {
        "Import image"
    };

    rsx! {
        main { "data-testid": "sheets-canvas",
            h2 { "{label}" }
        }
    }
}
