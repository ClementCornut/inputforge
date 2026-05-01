//! Layout primitives. Token-only API: `gap`/`padding` accept a CSS
//! custom-property *name* (e.g. `"--space-4"`), not a raw px value, so
//! magic geometry never enters consumer code.
//!
//! **Stale-property guard.** Each primitive ALWAYS emits every CSS
//! custom property it owns, using `initial` as a sentinel when the prop
//! was not supplied. Dioxus 0.7 patches inline `style` per property, so
//! a Some -> None transition would otherwise leave the previous value
//! lingering in the DOM. With `initial`, `var(--stack-gap, fallback)`
//! resolves to the fallback regardless of prior renders.

use dioxus::prelude::*;

use super::merge_class;

/// Format a `var(...)` reference for a custom-property reference, or
/// the literal `initial` sentinel when no token was supplied. The
/// sentinel is what makes the per-render style string idempotent across
/// Some / None transitions.
fn var_or_initial(token: Option<&str>) -> String {
    match token {
        Some(t) => format!("var({t})"),
        None => "initial".to_owned(),
    }
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
    let style = format!(
        "--stack-gap: {gap}; --stack-padding: {padding};",
        gap = var_or_initial(gap.as_deref()),
        padding = var_or_initial(padding.as_deref()),
    );
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
    let style = format!(
        "--cluster-gap: {gap}; --cluster-padding: {padding};",
        gap = var_or_initial(gap.as_deref()),
        padding = var_or_initial(padding.as_deref()),
    );
    rsx! {
        div { class: "{combined}", style: "{style}", {children} }
    }
}

/// Padded box. No layout opinion, just a token-driven inset.
#[component]
pub fn Inset(
    #[props(default)] padding: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-inset", "", class.as_deref());
    let style = format!(
        "--inset-padding: {padding};",
        padding = var_or_initial(padding.as_deref()),
    );
    rsx! {
        div { class: "{combined}", style: "{style}", {children} }
    }
}
