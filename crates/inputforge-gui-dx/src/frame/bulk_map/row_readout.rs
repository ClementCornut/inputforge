//! Compact live readout per row.
//!
//! Reads from `live.device_inputs` via the helpers shared with F9's
//! `LiveReadout` (`read_axis_display`, `read_button_pressed`,
//! `read_hat_direction`). The wizard renders these values into its
//! own grid template; F9 keeps its editor-row template.
//!
//! Rendered per row kind:
//! - Axis: bipolar bar (centered at 50%; fill grows toward the active edge).
//! - Button: filled-or-stamped dot.
//! - Hat: mono cardinal letter (N/E/S/W/NE/SE/SW/NW/centered dot).

use dioxus::prelude::*;

use inputforge_core::types::{HatDirection, InputAddress};

use crate::context::AppContext;
use crate::frame::bulk_map::state::RowKind;
use crate::frame::mapping_editor::live_readout::{
    AxisPolarity, read_axis_display, read_button_pressed, read_hat_direction,
};

#[component]
pub(super) fn RowReadout(kind: RowKind, address: InputAddress) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();

    match kind {
        RowKind::Axis => {
            let display = read_axis_display(&address, &live, &cfg);
            let value = display.value;
            let half_width = (value.abs() * 50.0).clamp(0.0, 50.0);
            let style = match display.polarity {
                AxisPolarity::Bipolar if value >= 0.0 => {
                    format!("left: 50%; right: auto; width: {half_width:.2}%")
                }
                AxisPolarity::Bipolar => {
                    format!("right: 50%; left: auto; width: {half_width:.2}%")
                }
                AxisPolarity::Unipolar => {
                    let pct = (value * 100.0).clamp(0.0, 100.0);
                    format!("left: 0; right: auto; width: {pct:.2}%")
                }
            };
            rsx! {
                div { class: "if-bulk-map__live if-bulk-map__live--axis",
                    div { class: "if-bulk-map__live-bar", style: "{style}" }
                }
            }
        }
        RowKind::Button => {
            let pressed = read_button_pressed(&address, &live, &cfg);
            let cls = if pressed {
                "if-bulk-map__live if-bulk-map__live--button if-bulk-map__live--button-on"
            } else {
                "if-bulk-map__live if-bulk-map__live--button"
            };
            rsx! { div { class: "{cls}" } }
        }
        RowKind::Hat => {
            let direction = read_hat_direction(&address, &live, &cfg);
            let label = match direction {
                HatDirection::Center => "·",
                HatDirection::N => "N",
                HatDirection::NE => "NE",
                HatDirection::E => "E",
                HatDirection::SE => "SE",
                HatDirection::S => "S",
                HatDirection::SW => "SW",
                HatDirection::W => "W",
                HatDirection::NW => "NW",
            };
            rsx! { div { class: "if-bulk-map__live if-bulk-map__live--hat", "{label}" } }
        }
    }
}
