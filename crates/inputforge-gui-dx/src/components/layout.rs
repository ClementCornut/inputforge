//! Layout primitives. Token-only API: `gap`/`padding` accept a CSS
//! custom-property *name* (e.g. `"--space-4"`), not a raw px value, so
//! magic geometry never enters consumer code.

use std::fmt::Write as _;

use dioxus::prelude::*;

use super::merge_class;

/// Format a `var(...)` reference if a token name was supplied, else `None`.
fn var_ref(token: Option<&str>) -> Option<String> {
    token.map(|t| format!("var({t})"))
}

/// Vertical stack: a `display: flex; flex-direction: column;` container
/// with token-driven gap and optional padding.
#[component]
pub fn Stack(
    /// CSS custom-property name (e.g. `"--space-4"`). Default: `--space-4`.
    #[props(default)]
    gap: Option<String>,
    /// CSS custom-property name (e.g. `"--space-6"`). Default: no padding.
    #[props(default)]
    padding: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-stack", "", class.as_deref());
    let mut style = String::new();
    if let Some(g) = var_ref(gap.as_deref()) {
        let _ = write!(style, "--stack-gap: {g};");
    }
    if let Some(p) = var_ref(padding.as_deref()) {
        let _ = write!(style, "--stack-padding: {p};");
    }
    rsx! {
        div { class: "{combined}", style: "{style}", {children} }
    }
}

/// Horizontal cluster: wraps children in a row, vertically centered, with
/// token-driven gap. Use for action rows, badge groups, breadcrumbs.
#[component]
pub fn Cluster(
    #[props(default)] gap: Option<String>,
    #[props(default)] padding: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-cluster", "", class.as_deref());
    let mut style = String::new();
    if let Some(g) = var_ref(gap.as_deref()) {
        let _ = write!(style, "--cluster-gap: {g};");
    }
    if let Some(p) = var_ref(padding.as_deref()) {
        let _ = write!(style, "--cluster-padding: {p};");
    }
    rsx! {
        div { class: "{combined}", style: "{style}", {children} }
    }
}

/// Padded box. No layout opinion — just a token-driven inset.
#[component]
pub fn Inset(
    #[props(default)] padding: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-inset", "", class.as_deref());
    let style = match var_ref(padding.as_deref()) {
        Some(p) => format!("--inset-padding: {p};"),
        None => String::new(),
    };
    rsx! {
        div { class: "{combined}", style: "{style}", {children} }
    }
}
