//! Native trigger presentations sharing action-family chrome.
use super::{MenuState, merge_class};
use dioxus::prelude::*;

/// Shared appearance for a native menu trigger; children contain no interactive controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuTriggerPresentation {
    #[default]
    Standard,
    Icon,
    Create,
}

/// Render one accessible native menu trigger, without a nested action button.
#[component]
pub fn MenuTrigger(
    #[props(default)] class: Option<String>,
    #[props(default)] unstyled: bool,
    #[props(default)] presentation: MenuTriggerPresentation,
    /// Small create-slot geometry; applies to the Create presentation.
    #[props(default)]
    compact: bool,
    /// Emphasized create-slot tone; applies to the Create presentation.
    #[props(default)]
    emphasized: bool,
    #[props(default)] disabled: bool,
    #[props(default)] title: Option<String>,
    #[props(default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let base = match presentation {
        MenuTriggerPresentation::Standard => "if-menu__trigger",
        MenuTriggerPresentation::Icon => "if-icon-button if-icon-button--ghost if-icon-button--sm",
        MenuTriggerPresentation::Create => match (compact, emphasized) {
            (false, false) => "if-button if-button--ghost if-button--md if-button--create",
            (true, false) => "if-button if-button--ghost if-button--sm if-button--create",
            (false, true) => "if-button if-button--ghost-primary if-button--md if-button--create",
            (true, true) => "if-button if-button--ghost-primary if-button--sm if-button--create",
        },
    };
    let combined = if unstyled {
        class.clone().unwrap_or_default()
    } else {
        merge_class(base, "", class.as_deref())
    };
    let menu_id = state.menu_id.read().clone();
    let trigger_id = state.trigger_id.read().clone();
    let toggle = state.behavior.toggle;
    let onclick = move |event| {
        if !disabled {
            toggle.call(event);
        }
    };
    rsx! {
        button {
            r#type: "button",
            class: combined,
            id: trigger_id,
            title,
            disabled,
            onclick,
            "aria-haspopup": "menu",
            "aria-expanded": state.open.read().to_string(),
            "aria-controls": menu_id,
            "aria-label": aria_label,
            {children}
        }
    }
}
