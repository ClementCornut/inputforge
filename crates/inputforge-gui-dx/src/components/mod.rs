//! Re-exports for the F2 component primitives.

pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod chip;
pub mod click_away_listener;
pub mod dialog;
pub mod drawer;
pub mod field;
pub mod icon;
pub mod icon_button;
pub mod label;
pub mod layout;
pub mod menu;
pub mod number_input;
pub mod portal;
pub mod select;
pub mod separator;
pub mod slider;
pub mod sortable;
pub mod spinner;
pub mod status_bar;
pub mod switch;
pub mod tab_button;
pub mod tabs;
pub mod tabs_list;
pub mod tabs_root;
pub mod text_input;
pub mod tooltip;

pub use badge::{Badge, BadgeVariant};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardPadding};
pub use checkbox::Checkbox;
pub use chip::{Chip, ChipVariant};
pub use click_away_listener::ClickAwayListener;
pub use dialog::{DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle};
pub use drawer::{Drawer, DrawerAnchor, DrawerVariant};
pub use field::Field;
pub use icon::Icon;
pub use icon_button::IconButton;
pub use label::Label;
pub use layout::{Cluster, Inset, Stack};
pub use menu::{
    Anchor, AnchoredMenu, CloseReason, MenuAnchor, MenuItem, MenuItems, MenuRoot, MenuTrigger,
};
pub use number_input::NumberInput;
pub use portal::Portal;
pub use select::{Select, SelectOption};
pub use separator::{Separator, SeparatorOrientation};
pub use slider::Slider;
pub use spinner::{Spinner, SpinnerSize};
pub use status_bar::StatusBar;
pub use switch::Switch;
pub use tab_button::TabButton;
pub use tabs::{TabItem, Tabs};
pub use tabs_list::{TabsList, TabsOrientation};
pub use tabs_root::TabsRoot;
pub use text_input::{InputSize, TextInput};
pub use tooltip::{Tooltip, TooltipPlacement};

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
    if let Some(c) = caller {
        if !c.is_empty() {
            out.push(' ');
            out.push_str(c);
        }
    }
    out
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
