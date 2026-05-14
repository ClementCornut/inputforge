#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use dioxus::prelude::*;

use crate::frame::sheets::state::SheetsState;

#[component]
pub(crate) fn SheetsCanvas(
    sheets: Signal<SheetsState>,
    on_import_image: EventHandler<()>,
) -> Element {
    let has_template = sheets.read().selected_template_id.is_some();
    let handle_import_image = move |_| on_import_image.call(());
    let label = if has_template {
        "Template image"
    } else {
        "Import image"
    };

    rsx! {
        main { "data-testid": "sheets-canvas",
            h2 { "{label}" }
            if !has_template {
                button {
                    "type": "button",
                    onclick: handle_import_image,
                    "Import image"
                }
            }
        }
    }
}
