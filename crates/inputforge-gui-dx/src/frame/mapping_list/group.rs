//! Bucket mappings by input kind. Render order is fixed AXES -> BUTTONS -> HATS.

use inputforge_core::types::{InputAddress, InputId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    Axes,
    Buttons,
    Hats,
}

impl GroupKind {
    /// Fixed render order. Iteration produces [`GroupKind::Axes`,
    /// `GroupKind::Buttons`, `GroupKind::Hats`] - empty groups are
    /// omitted at render time, but ordering between the surviving
    /// groups never changes.
    pub(crate) const fn ordered() -> [GroupKind; 3] {
        [GroupKind::Axes, GroupKind::Buttons, GroupKind::Hats]
    }

    /// Header label for a group. UPPER-CASE per the F8 wireframe.
    pub(crate) const fn header(self) -> &'static str {
        match self {
            GroupKind::Axes => "AXES",
            GroupKind::Buttons => "BUTTONS",
            GroupKind::Hats => "HATS",
        }
    }
}

pub(crate) fn group_of(addr: &InputAddress) -> GroupKind {
    let input = addr
        .input_id()
        .expect("invariant: mapping list group addr always bound (mapping primary)");
    match input {
        InputId::Axis { .. } => GroupKind::Axes,
        InputId::Button { .. } => GroupKind::Buttons,
        InputId::Hat { .. } => GroupKind::Hats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::DeviceId;

    fn addr(input: InputId) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input,
        }
    }

    #[test]
    fn group_of_axis_maps_to_axes() {
        assert_eq!(group_of(&addr(InputId::Axis { index: 0 })), GroupKind::Axes);
    }

    #[test]
    fn group_of_button_maps_to_buttons() {
        assert_eq!(
            group_of(&addr(InputId::Button { index: 0 })),
            GroupKind::Buttons,
        );
    }

    #[test]
    fn group_of_hat_maps_to_hats() {
        assert_eq!(group_of(&addr(InputId::Hat { index: 0 })), GroupKind::Hats);
    }

    #[test]
    fn ordered_returns_axes_buttons_hats() {
        assert_eq!(
            GroupKind::ordered(),
            [GroupKind::Axes, GroupKind::Buttons, GroupKind::Hats],
        );
    }

    #[test]
    fn header_labels_upper_case() {
        assert_eq!(GroupKind::Axes.header(), "AXES");
        assert_eq!(GroupKind::Buttons.header(), "BUTTONS");
        assert_eq!(GroupKind::Hats.header(), "HATS");
    }
}
