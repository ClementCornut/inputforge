//! Mounts the F2 design-system stylesheets and exposes them to descendants.
//!
//! Stylesheet load order (cascade priority, lowest first):
//! tokens → global → components. Order matters — do not reshuffle.

use dioxus::prelude::*;

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! { {children} }
}
