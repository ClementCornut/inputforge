//! Reusable design-system primitives for the `InputForge` renderer.
//!
//! Import primitives from this module; sortable composition lives in [`sortable`].

mod actions;
mod feedback;
mod inputs;
mod interaction;
mod layout;
mod navigation;
mod overlays;

#[doc(inline)]
pub use actions::button::{Button, ButtonProps, ButtonSize, ButtonVariant};
#[doc(inline)]
pub use actions::icon_button::{IconButton, IconButtonProps};

#[doc(inline)]
pub use inputs::checkbox::{Checkbox, CheckboxProps};
#[doc(inline)]
pub use inputs::integer_input::{IntegerInput, IntegerInputError, IntegerInputProps};
#[doc(inline)]
pub use inputs::number_input::{NumberInput, NumberInputProps};
#[doc(inline)]
pub use inputs::segmented_control::{
    SegmentedControl, SegmentedControlOption, SegmentedControlOptionProps, SegmentedControlProps,
};
#[doc(inline)]
pub use inputs::select::{Select, SelectOption, SelectProps};
#[doc(inline)]
pub use inputs::slider::{Slider, SliderProps};
#[doc(inline)]
pub use inputs::switch::{Switch, SwitchProps};
#[doc(inline)]
pub use inputs::text_input::{InputSize, TextInput, TextInputProps};

#[doc(inline)]
pub use layout::card::{Card, CardPadding, CardProps};
#[doc(inline)]
pub use layout::drawer::{Drawer, DrawerAnchor, DrawerProps, DrawerVariant};
#[doc(inline)]
pub use layout::field::{Field, FieldProps};
#[doc(inline)]
pub use layout::label::{Label, LabelProps};
#[doc(inline)]
pub use layout::separator::{Separator, SeparatorOrientation, SeparatorProps};
#[doc(inline)]
pub use layout::spacing::{Cluster, ClusterProps, Inset, InsetProps, Stack, StackProps};

#[doc(inline)]
pub use navigation::menu::{
    Anchor, AnchoredMenu, AnchoredMenuProps, MenuAnchor, MenuItem, MenuItemProps, MenuItems,
    MenuItemsProps, MenuRoot, MenuRootProps, MenuTrigger, MenuTriggerProps,
};
#[doc(inline)]
pub use navigation::tab_button::{TabButton, TabButtonProps};
#[doc(inline)]
pub use navigation::tabs::{TabItem, Tabs, TabsProps};
#[doc(inline)]
pub use navigation::tabs_list::{TabsList, TabsListProps, TabsOrientation};
#[doc(inline)]
pub use navigation::tabs_root::{TabsRoot, TabsRootProps};

#[doc(inline)]
pub use feedback::badge::{Badge, BadgeProps, BadgeVariant};
#[doc(inline)]
pub use feedback::chip::{Chip, ChipProps, ChipVariant};
#[doc(inline)]
pub use feedback::icon::{Icon, IconProps};
#[doc(inline)]
pub use feedback::spinner::{Spinner, SpinnerProps, SpinnerSize};
#[doc(inline)]
pub use feedback::status_bar::{StatusBar, StatusBarProps};
#[doc(inline)]
pub use feedback::tooltip::{Tooltip, TooltipPlacement, TooltipProps};

#[doc(inline)]
pub use overlays::click_away_listener::{ClickAwayListener, ClickAwayListenerProps};
#[doc(inline)]
pub use overlays::dialog::{
    DialogBody, DialogBodyProps, DialogDescription, DialogDescriptionProps, DialogFooter,
    DialogFooterProps, DialogRoot, DialogRootProps, DialogTitle, DialogTitleProps,
};
#[doc(inline)]
pub use overlays::portal::{Portal, PortalProps};

#[doc(inline)]
pub use interaction::sortable;

/// Joins a base class, a variant class, and an optional caller class.
/// Used by every primitive to honor the `class: Option<String>` prop.
/// Empty parts are skipped so primitives without a size/variant modifier
/// (Slider, Label, Field, etc.) may pass `""` as the variant.
pub(crate) fn merge_class(base: &str, variant: &str, caller: Option<&str>) -> String {
    let mut out = String::from(base);
    if !variant.is_empty() {
        out.push(' ');
        out.push_str(variant);
    }
    if let Some(c) = caller
        && !c.is_empty()
    {
        out.push(' ');
        out.push_str(c);
    }
    out
}

/// Resolve the host behavior; unit fixtures install the canonical app implementation.
pub(crate) fn behavior() -> crate::controls::Bindings {
    use dioxus::prelude::use_context;
    #[cfg(test)]
    use dioxus::prelude::{try_consume_context, use_context_provider};
    #[cfg(test)]
    use_context_provider(|| try_consume_context().unwrap_or_else(inputforge_app::ui::bindings));
    use_context()
}

#[cfg(test)]
mod tests {
    use super::merge_class;

    #[test]
    fn with_caller() {
        assert_eq!(merge_class("a", "b", Some("c")), "a b c");
    }
    #[test]
    fn without_caller() {
        assert_eq!(merge_class("a", "b", None), "a b");
    }
    #[test]
    fn empty_caller() {
        assert_eq!(merge_class("a", "b", Some("")), "a b");
    }
    #[test]
    fn empty_variant() {
        assert_eq!(merge_class("a", "", Some("c")), "a c");
    }
    #[test]
    fn empty_variant_no_caller() {
        assert_eq!(merge_class("a", "", None), "a");
    }
    #[test]
    fn no_trailing_space() {
        assert!(!merge_class("a", "b", None).ends_with(' '));
    }
}
