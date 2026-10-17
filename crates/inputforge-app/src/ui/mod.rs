//! Shared, engine-independent behavior for controls and overlay rendering.
mod draft;
mod inputs;
mod integer;
mod overlays;
mod tabs;
use dioxus::prelude::*;
use inputforge_gui_dx::controls::Bindings;
/// Construct the real control and overlay callbacks for an editor, gallery or fixture.
#[must_use]
pub fn bindings() -> Bindings {
    Bindings {
        overlays: overlays::bindings(),
        number: inputs::use_number,
        integer: integer::use_integer,
        text_key: Callback::new(
            |(event, disabled, handler): (
                KeyboardEvent,
                bool,
                Option<EventHandler<KeyboardEvent>>,
            )| {
                if disabled {
                    return;
                }
                if matches!(
                    event.key(),
                    Key::ArrowLeft
                        | Key::ArrowRight
                        | Key::ArrowUp
                        | Key::ArrowDown
                        | Key::Home
                        | Key::End
                ) {
                    event.stop_propagation();
                }
                if let Some(handler) = handler {
                    handler.call(event);
                }
            },
        ),
        segmented_click: Callback::new(
            |(event, disabled, handler): (MouseEvent, bool, Option<EventHandler<MouseEvent>>)| {
                if !disabled && let Some(handler) = handler {
                    handler.call(event);
                }
            },
        ),
        tabs: tabs::use_tabs,
    }
}
/// Install the same application behavior without engine or desktop services.
pub fn use_provider() {
    use_context_provider(bindings);
}
