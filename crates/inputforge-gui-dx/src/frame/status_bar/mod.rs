mod logic;

use dioxus::prelude::*;

use crate::components::{Badge, BadgeVariant, StatusBar as StatusBarPrimitive};
use crate::context::AppContext;

use logic::{device_count_label, truncate_path, warning_count_label};

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: STATUS_BAR_CSS }"
)]
const STATUS_BAR_CSS: Asset = asset!("/assets/frame/status_bar.css");

/// Maximum characters shown in the profile-path slot before truncation.
const PATH_TRUNCATE_BUDGET: usize = 64;

/// F7 status bar: device count (middle), optional warning badge (start),
/// profile path or em-dash (end).
#[component]
pub(crate) fn StatusBar() -> Element {
    let ctx = use_context::<AppContext>();

    let devices_label = use_memo(move || device_count_label(&ctx.config.read().devices));
    let warning_label = use_memo(move || warning_count_label(ctx.meta.read().warnings.len()));
    let path_label = use_memo(move || {
        ctx.meta
            .read()
            .profile_path
            .as_ref()
            .map(|p| truncate_path(p, PATH_TRUNCATE_BUDGET))
    });

    let d = devices_label.read().clone();
    let w = warning_label.read().clone();
    let p = path_label.read().clone();

    rsx! {
        Stylesheet { href: STATUS_BAR_CSS }
        StatusBarPrimitive {
            class: "if-frame-status-bar".to_owned(),
            start: rsx! {
                if let Some(text) = w.as_ref() {
                    Badge { variant: BadgeVariant::Warning, "{text}" }
                }
            },
            middle: rsx! { span { "{d}" } },
            end: rsx! {
                match p {
                    Some(s) => rsx! { span { class: "if-frame-status-bar__path", "{s}" } },
                    None    => rsx! { span { class: "if-frame-status-bar__path-empty", "—" } },
                }
            },
        }
    }
}
