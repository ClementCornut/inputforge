//! Mounts the F2 design-system stylesheets and exposes them to descendants.
//!
//! Stylesheet load order (cascade priority, lowest first):
//!     tokens → global → components.
//! `document::Stylesheet` mounts in render order, so the rsx! sequence
//! below IS the cascade order. Do not reshuffle.

use dioxus::prelude::*;

const COLORS_CSS: Asset = asset!("/assets/tokens/colors.css");
const TYPOGRAPHY_CSS: Asset = asset!("/assets/tokens/typography.css");
const SPACING_CSS: Asset = asset!("/assets/tokens/spacing.css");
const RADII_CSS: Asset = asset!("/assets/tokens/radii.css");
const ELEVATION_CSS: Asset = asset!("/assets/tokens/elevation.css");
const MOTION_CSS: Asset = asset!("/assets/tokens/motion.css");
const GLOBAL_CSS: Asset = asset!("/assets/global.css");
const ICON_CSS: Asset = asset!("/assets/components/icon.css");
const BUTTON_CSS: Asset = asset!("/assets/components/button.css");
const ICON_BUTTON_CSS: Asset = asset!("/assets/components/icon-button.css");
const TEXT_INPUT_CSS: Asset = asset!("/assets/components/text-input.css");
const NUMBER_INPUT_CSS: Asset = asset!("/assets/components/number-input.css");
const SELECT_CSS: Asset = asset!("/assets/components/select.css");
const SLIDER_CSS: Asset = asset!("/assets/components/slider.css");
const SWITCH_CSS: Asset = asset!("/assets/components/switch.css");
const CHECKBOX_CSS: Asset = asset!("/assets/components/checkbox.css");
const CARD_CSS: Asset = asset!("/assets/components/card.css");
const BADGE_CSS: Asset = asset!("/assets/components/badge.css");
const SEPARATOR_CSS: Asset = asset!("/assets/components/separator.css");
const SPINNER_CSS: Asset = asset!("/assets/components/spinner.css");

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! {
        // Tokens first (lowest cascade priority).
        Stylesheet { href: COLORS_CSS }
        Stylesheet { href: TYPOGRAPHY_CSS }
        Stylesheet { href: SPACING_CSS }
        Stylesheet { href: RADII_CSS }
        Stylesheet { href: ELEVATION_CSS }
        Stylesheet { href: MOTION_CSS }

        // Body baseline.
        Stylesheet { href: GLOBAL_CSS }

        // Component CSS will be appended here as primitives land (Tasks 13-20).
        Stylesheet { href: ICON_CSS }
        Stylesheet { href: BUTTON_CSS }
        Stylesheet { href: ICON_BUTTON_CSS }
        Stylesheet { href: TEXT_INPUT_CSS }
        Stylesheet { href: NUMBER_INPUT_CSS }
        Stylesheet { href: SELECT_CSS }
        Stylesheet { href: SLIDER_CSS }
        Stylesheet { href: SWITCH_CSS }
        Stylesheet { href: CHECKBOX_CSS }
        Stylesheet { href: CARD_CSS }
        Stylesheet { href: BADGE_CSS }
        Stylesheet { href: SEPARATOR_CSS }
        Stylesheet { href: SPINNER_CSS }

        {children}
    }
}
