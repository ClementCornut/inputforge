//! Re-exports for the F2 component primitives.

pub mod button;
pub mod icon;
pub mod icon_button;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use icon::Icon;
pub use icon_button::IconButton;

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
