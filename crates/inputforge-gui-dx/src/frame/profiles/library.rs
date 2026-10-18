use crate::{
    components::{
        Button, ButtonPresentation, ButtonSize, ButtonVariant, Icon, IconButton, InputSize,
        TextInput,
    },
    icons::Icon as IconKind,
    profiles::{Context, Intent},
};
use dioxus::prelude::*;
#[component]
pub(crate) fn ProfileLibrary() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let filter = use_memo(move || c.view.read().filter.clone());
    let new_click = move |_| {
        let _ = c.send(Intent::New);
    };
    rsx! {
        div {
            class: "profiles-panel__library",
            div {
                class: "profiles-panel__filter",
                TextInput {
                    value: filter,
                    aria_label: "Filter profiles",
                    class: "profiles-panel__filter-input",
                    size: InputSize::Sm,
                    placeholder: "Filter profiles".to_owned(),
                    oninput: move |e: FormEvent| {
                        let _ = c.send(Intent::Filter(e.value()));
                    },
                }
                IconButton {
                    icon: IconKind::FolderOpen,
                    label: "Open profile from file",
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "profiles-panel__open-file".to_owned(),
                    onclick: move |_| {
                        let _ = c.send(Intent::Pick);
                    },
                }
            }
            for row in view.rows {
                super::row::ProfileRow {
                    key: "{row.target.key}",
                    row,
                }
            }
            if view.filtered_empty {
                crate::components::InlineFeedback { class: "profile-row__filtered-empty", "No matching profiles." }
            }
            Button {
                presentation: ButtonPresentation::Create, variant: ButtonVariant::Ghost,
                onclick: new_click,
                Icon {
                    name: IconKind::Plus,
                }
                "New profile"
            }
        }
    }
}
