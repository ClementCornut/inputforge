//! Mounts the F2 design-system stylesheets and exposes them to descendants.
//!
//! Stylesheet load order (cascade priority, lowest first):
//!     tokens → global → components.
//! `document::Stylesheet` mounts in render order, so the rsx! sequence
//! below IS the cascade order. Do not reshuffle.

use dioxus::prelude::*;

const INTER_REGULAR: Asset = asset!("/assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: Asset = asset!("/assets/fonts/Inter-SemiBold.ttf");
const INTER_BOLD: Asset = asset!("/assets/fonts/Inter-Bold.ttf");
const JETBRAINS_MONO: Asset = asset!("/assets/fonts/JetBrainsMono-Regular.ttf");

const COLORS_CSS: Asset = asset!("/assets/tokens/colors.css");
const TYPOGRAPHY_CSS: Asset = asset!("/assets/tokens/typography.css");
const SPACING_CSS: Asset = asset!("/assets/tokens/spacing.css");
const RADII_CSS: Asset = asset!("/assets/tokens/radii.css");
const ELEVATION_CSS: Asset = asset!("/assets/tokens/elevation.css");
const MOTION_CSS: Asset = asset!("/assets/tokens/motion.css");
const INSTRUMENTS_TOKENS_CSS: Asset = asset!("/assets/tokens/instruments.css");
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
const DIALOG_CSS: Asset = asset!("/assets/components/dialog.css");
const CARD_CSS: Asset = asset!("/assets/components/card.css");
const BADGE_CSS: Asset = asset!("/assets/components/badge.css");
const SEPARATOR_CSS: Asset = asset!("/assets/components/separator.css");
const SPINNER_CSS: Asset = asset!("/assets/components/spinner.css");
const TOOLTIP_CSS: Asset = asset!("/assets/components/tooltip.css");
const MENU_CSS: Asset = asset!("/assets/components/menu.css");
const LABEL_CSS: Asset = asset!("/assets/components/label.css");
const FIELD_CSS: Asset = asset!("/assets/components/field.css");
const LAYOUT_CSS: Asset = asset!("/assets/components/layout.css");
const TABS_CSS: Asset = asset!("/assets/components/tabs.css");
const STATUS_BAR_CSS: Asset = asset!("/assets/components/status-bar.css");
const SORTABLE_CSS: Asset = asset!("/assets/components/sortable.css");
const TOAST_CSS: Asset = asset!("/assets/toast/toast.css");

const RESPONSE_CURVE_CSS: Asset = asset!("/assets/frame/response_curve.css");
const DEADZONE_CSS: Asset = asset!("/assets/frame/deadzone.css");

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    // @font-face rules need manganis-bundled asset URLs interpolated at
    // runtime, manganis 0.6 does NOT rewrite url() refs inside asset!()'d CSS,
    // so font URLs hardcoded in typography.css 404 on load. Inline <style>
    // here is the only way to get the hashed bundled URL into a font-face src.
    let font_faces = format!(
        "@font-face {{ font-family: 'Inter'; src: url('{INTER_REGULAR}') format('truetype'); font-weight: 400; font-display: swap; }}\
         @font-face {{ font-family: 'Inter'; src: url('{INTER_SEMIBOLD}') format('truetype'); font-weight: 600; font-display: swap; }}\
         @font-face {{ font-family: 'Inter'; src: url('{INTER_BOLD}') format('truetype'); font-weight: 700; font-display: swap; }}\
         @font-face {{ font-family: 'JetBrainsMono'; src: url('{JETBRAINS_MONO}') format('truetype'); font-weight: 400; font-display: swap; }}"
    );

    rsx! {
        style { "{font_faces}" }

        // Tokens first (lowest cascade priority).
        Stylesheet { href: COLORS_CSS }
        Stylesheet { href: TYPOGRAPHY_CSS }
        Stylesheet { href: SPACING_CSS }
        Stylesheet { href: RADII_CSS }
        Stylesheet { href: ELEVATION_CSS }
        Stylesheet { href: MOTION_CSS }
        Stylesheet { href: INSTRUMENTS_TOKENS_CSS }

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
        Stylesheet { href: DIALOG_CSS }
        Stylesheet { href: CARD_CSS }
        Stylesheet { href: BADGE_CSS }
        Stylesheet { href: SEPARATOR_CSS }
        Stylesheet { href: SPINNER_CSS }
        Stylesheet { href: TOOLTIP_CSS }
        Stylesheet { href: MENU_CSS }
        Stylesheet { href: LABEL_CSS }
        Stylesheet { href: FIELD_CSS }
        Stylesheet { href: LAYOUT_CSS }
        Stylesheet { href: TABS_CSS }
        Stylesheet { href: STATUS_BAR_CSS }
        Stylesheet { href: SORTABLE_CSS }

        // Frame stylesheets (editor panels, curve editor, etc.).
        Stylesheet { href: RESPONSE_CURVE_CSS }
        Stylesheet { href: DEADZONE_CSS }

        // Toast overlay, last so its z-index cascade wins.
        Stylesheet { href: TOAST_CSS }

        {children}
    }
}
