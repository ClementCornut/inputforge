// Rust guideline compliant 2026-03-04

//! Dual-theme cockpit palette, Inter font stack, and semantic color system.
//!
//! Provides a "Glass Cockpit" aesthetic in both dark and light variants.
//! The dark theme uses saturated accents on navy-tinted backgrounds; the
//! light "Daylight" variant mirrors that blue undertone with vivid,
//! WCAG-adjusted accents. Both themes follow OS preference by default
//! (`ThemePreference::System`).
//!
//! All widget and panel code should use [`colors`] to obtain the active
//! palette rather than hard-coding color values.

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Theme, Visuals};

// ---------------------------------------------------------------------------
// Semantic color palette
// ---------------------------------------------------------------------------

/// Complete semantic color palette for one theme variant.
///
/// Holds background layers, text colors, and accent colors. Two const
/// instances ([`DARK`] and [`LIGHT`]) cover the built-in themes; the
/// struct is designed to be loaded from user configuration in the future.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThemeColors {
    // Background layers (ordered by depth: base deepest, surface1 shallowest).
    /// Primary window and panel background.
    pub base: Color32,
    /// Slightly recessed layer (e.g., scroll-area backdrop).
    pub mantle: Color32,
    /// Deepest recessed surface (e.g., text input backgrounds).
    pub crust: Color32,
    /// Elevated widget background (cards, button fills).
    pub surface0: Color32,
    /// Border and separator strokes.
    pub surface1: Color32,

    // Text.
    /// Primary body text.
    pub text: Color32,
    /// Dimmed / secondary text (labels, placeholders).
    pub text_dim: Color32,

    // Accents.
    /// Primary interactive accent (buttons, links, selections).
    pub primary: Color32,
    /// Live hardware data accent (axis fills, active buttons).
    pub live: Color32,
    /// Warning / caution accent (amber).
    pub warning: Color32,
    /// Error / destructive accent (red).
    pub error: Color32,
    /// Special accent for modes and conditions (purple).
    pub special: Color32,
    /// Idle / inactive graphical indicator (compass triangles, button rings).
    pub indicator_idle: Color32,
}

impl ThemeColors {
    /// Processing action color (response curve, deadzone, calibrate, invert).
    pub(crate) const fn processing(&self) -> Color32 {
        self.primary
    }

    /// Output action color (map-to-vJoy, map-to-keyboard, merge-axis).
    pub(crate) const fn output(&self) -> Color32 {
        self.live
    }

    /// Control flow action color (change-mode, conditional).
    pub(crate) const fn control(&self) -> Color32 {
        self.special
    }

    /// Negative active zone fill (amber-tinted, dimmed).
    pub(crate) fn zone_negative(&self) -> Color32 {
        self.warning.gamma_multiply(0.25)
    }

    /// Positive active zone fill (blue-tinted, dimmed).
    pub(crate) fn zone_positive(&self) -> Color32 {
        self.primary.gamma_multiply(0.25)
    }

    /// Out-of-range saturated zone fill (red-tinted, dimmed).
    pub(crate) fn zone_saturated(&self) -> Color32 {
        self.error.gamma_multiply(0.5)
    }

    /// Semi-transparent overlay for disabled state.
    pub(crate) fn disabled_overlay(&self) -> Color32 {
        self.surface0.gamma_multiply(0.7)
    }
}

// ---------------------------------------------------------------------------
// Built-in palettes
// ---------------------------------------------------------------------------

/// Dark cockpit palette — navy-tinted backgrounds with bright accents.
pub(crate) const DARK: ThemeColors = ThemeColors {
    base: Color32::from_rgb(0x1A, 0x1A, 0x2E),
    mantle: Color32::from_rgb(0x16, 0x16, 0x3A),
    crust: Color32::from_rgb(0x12, 0x12, 0x28),
    surface0: Color32::from_rgb(0x2A, 0x2A, 0x3E),
    surface1: Color32::from_rgb(0x3A, 0x3A, 0x4E),
    text: Color32::from_rgb(0xE0, 0xE0, 0xE8),
    text_dim: Color32::from_rgb(0xA0, 0xA0, 0xB8),
    primary: Color32::from_rgb(0x4A, 0x9E, 0xFF),
    live: Color32::from_rgb(0x00, 0xE5, 0xA0),
    warning: Color32::from_rgb(0xFF, 0xB3, 0x47),
    error: Color32::from_rgb(0xFF, 0x6B, 0x6B),
    special: Color32::from_rgb(0xB0, 0x7F, 0xFF),
    indicator_idle: Color32::from_rgb(0x55, 0x55, 0x70),
};

/// Light "Glass Cockpit Daylight" palette — blue-tinted whites with vivid,
/// WCAG-darkened accents.
///
/// Design: same instrument, different lighting. The dark theme's navy
/// undertone (#1A1A2E) is mirrored as a blue-tinted light base. Dark-base
/// becomes light-text for intentional symmetry.
pub(crate) const LIGHT: ThemeColors = ThemeColors {
    base: Color32::from_rgb(0xEC, 0xEE, 0xF6),
    mantle: Color32::from_rgb(0xE2, 0xE4, 0xEE),
    crust: Color32::from_rgb(0xD6, 0xD8, 0xE4),
    surface0: Color32::from_rgb(0xF6, 0xF7, 0xFC),
    surface1: Color32::from_rgb(0xCA, 0xCC, 0xE0),
    // Intentional symmetry: dark base = light text.
    text: Color32::from_rgb(0x1A, 0x1A, 0x2E),
    text_dim: Color32::from_rgb(0x5A, 0x5A, 0x72),
    // Accents darkened for WCAG contrast on light backgrounds,
    // but kept saturated to preserve the cockpit personality.
    primary: Color32::from_rgb(0x1B, 0x6B, 0xE0),
    live: Color32::from_rgb(0x00, 0x89, 0x6A),
    warning: Color32::from_rgb(0xB8, 0x74, 0x00),
    error: Color32::from_rgb(0xC8, 0x30, 0x30),
    special: Color32::from_rgb(0x6B, 0x38, 0xB8),
    indicator_idle: Color32::from_rgb(0x94, 0x94, 0xAC),
};

// ---------------------------------------------------------------------------
// Runtime accessor
// ---------------------------------------------------------------------------

/// Return the active color palette for the current egui theme.
///
/// Widgets and panels call this once per function to obtain theme-aware
/// colors without branching on every color reference.
pub(crate) fn colors(ctx: &egui::Context) -> &'static ThemeColors {
    match ctx.theme() {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

// ---------------------------------------------------------------------------
// Theme setup
// ---------------------------------------------------------------------------

/// Font size for secondary labels, status text, and compact UI elements.
pub(crate) const SMALL_FONT_SIZE: f32 = 12.0;

/// Widget corner radius in logical pixels.
const WIDGET_ROUNDING: u8 = 6;

/// Apply the dual cockpit theme: fonts, visuals for both dark and light
/// slots, and shared spacing tweaks.
///
/// Must be called from `InputForgeApp::new()` using the
/// `eframe::CreationContext` — fonts are unavailable in `update()`.
pub(crate) fn setup(ctx: &egui::Context) {
    load_fonts(ctx);

    // Apply custom visuals to both theme slots so the OS preference
    // (ThemePreference::System, the egui default) works correctly.
    ctx.set_visuals_of(Theme::Dark, build_visuals(&DARK, Visuals::dark()));
    ctx.set_visuals_of(Theme::Light, build_visuals(&LIGHT, Visuals::light()));

    // Spacing tweaks apply to both themes.
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
    });
}

/// Build egui [`Visuals`] from a [`ThemeColors`] palette.
///
/// Starts from `base_visuals` (typically `Visuals::dark()` or
/// `Visuals::light()`) and overrides colors to match the palette.
fn build_visuals(palette: &ThemeColors, mut visuals: Visuals) -> Visuals {
    // Global overrides.
    visuals.override_text_color = Some(palette.text);
    visuals.panel_fill = palette.base;
    visuals.window_fill = palette.base;
    visuals.extreme_bg_color = palette.crust;
    visuals.faint_bg_color = palette.mantle;
    visuals.selection.bg_fill = palette.primary.gamma_multiply(0.35);
    visuals.selection.stroke.color = palette.primary;
    visuals.hyperlink_color = palette.primary;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.error;
    visuals.code_bg_color = palette.crust;
    visuals.window_stroke = egui::Stroke::new(1.0, palette.surface1);

    // Widget states.
    let corner_radius = CornerRadius::same(WIDGET_ROUNDING);

    visuals.widgets.noninteractive.bg_fill = palette.surface0;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface0;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.surface1);
    visuals.widgets.noninteractive.fg_stroke.color = palette.text_dim;
    visuals.widgets.noninteractive.corner_radius = corner_radius;

    visuals.widgets.inactive.bg_fill = palette.surface0;
    visuals.widgets.inactive.weak_bg_fill = palette.surface0;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.surface1);
    visuals.widgets.inactive.fg_stroke.color = palette.text;
    visuals.widgets.inactive.corner_radius = corner_radius;

    visuals.widgets.hovered.bg_fill = palette.surface1;
    visuals.widgets.hovered.weak_bg_fill = palette.surface1;
    visuals.widgets.hovered.fg_stroke.color = palette.text;
    visuals.widgets.hovered.corner_radius = corner_radius;

    visuals.widgets.active.bg_fill = palette.primary.gamma_multiply(0.25);
    visuals.widgets.active.weak_bg_fill = palette.primary.gamma_multiply(0.25);
    visuals.widgets.active.fg_stroke.color = palette.primary;
    visuals.widgets.active.corner_radius = corner_radius;

    visuals.widgets.open.bg_fill = palette.surface1;
    visuals.widgets.open.weak_bg_fill = palette.surface1;
    visuals.widgets.open.fg_stroke.color = palette.text;
    visuals.widgets.open.corner_radius = corner_radius;

    visuals
}

/// Register Inter (proportional) and `JetBrains` Mono (monospace).
fn load_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "Inter-Regular".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "JetBrainsMono-Regular".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    // Primary proportional font.
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .expect("proportional family must exist")
        .insert(0, "Inter-Regular".to_owned());

    // Monospace font.
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .expect("monospace family must exist")
        .insert(0, "JetBrainsMono-Regular".to_owned());

    // Custom "SemiBold" family for headings.
    fonts.families.insert(
        FontFamily::Name("SemiBold".into()),
        vec!["Inter-SemiBold".to_owned()],
    );

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_does_not_panic() {
        let ctx = egui::Context::default();
        setup(&ctx);
    }

    /// All accent and indicator colors must be distinct within each palette.
    fn assert_accents_distinct(palette: &ThemeColors) {
        let accents = [
            palette.primary,
            palette.live,
            palette.warning,
            palette.error,
            palette.special,
            palette.indicator_idle,
        ];
        for (i, a) in accents.iter().enumerate() {
            for (j, b) in accents.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "accents at index {i} and {j} must differ");
                }
            }
        }
    }

    #[test]
    fn dark_accents_are_distinct() {
        assert_accents_distinct(&DARK);
    }

    #[test]
    fn light_accents_are_distinct() {
        assert_accents_distinct(&LIGHT);
    }

    #[test]
    fn dark_and_light_bases_differ() {
        assert_ne!(DARK.base, LIGHT.base);
    }

    #[test]
    fn text_symmetry_dark_base_is_light_text() {
        assert_eq!(DARK.base, LIGHT.text);
    }

    #[test]
    fn action_category_aliases() {
        assert_eq!(DARK.processing(), DARK.primary);
        assert_eq!(DARK.output(), DARK.live);
        assert_eq!(DARK.control(), DARK.special);
    }

    #[test]
    fn colors_returns_dark_for_dark_theme() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::ThemePreference::Dark);
        let c = colors(&ctx);
        assert_eq!(c.base, DARK.base);
    }

    #[test]
    fn colors_returns_light_for_light_theme() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::ThemePreference::Light);
        let c = colors(&ctx);
        assert_eq!(c.base, LIGHT.base);
    }
}
