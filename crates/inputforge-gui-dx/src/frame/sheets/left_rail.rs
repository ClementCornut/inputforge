use dioxus::prelude::*;

use crate::frame::sheets::state::SheetsState;

#[component]
pub(crate) fn SheetsLeftRail(sheets: Signal<SheetsState>) -> Element {
    let template_count = sheets.read().templates.len();

    rsx! {
        aside { "data-testid": "sheets-left-rail",
            h2 { "Templates" }
            p { "{template_count} templates" }
        }
    }
}
