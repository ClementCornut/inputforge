#![expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for native event attributes"
)]
use crate::{
    components::{ButtonSize, ButtonVariant, Icon, IconButton, InputSize, TextInput},
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
                div {
                    class: "profile-row__filtered-empty",
                    "No matching profiles."
                }
            }
            button {
                r#type: "button",
                class: "profile-row profile-row--create",
                onclick: new_click,
                Icon {
                    name: IconKind::Plus,
                }
                span {
                    class: "profile-row--create__label",
                    "New profile"
                }
            }
        }
    }
}
