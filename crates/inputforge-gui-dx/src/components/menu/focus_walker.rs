//! Shared focus-walker for menu-like role=menu structures. Reused by the
//! F2 dropdown menu and the F7 mode-tab context menu.
//!
//! The body is the same JS that previously lived inline in `components/menu.rs`
//! as `MENU_FOCUS_JS`. Behavior is identical: walks `[role=menuitem]:not(:disabled)`
//! descendants of `menu_id`, finds the currently-focused element among them,
//! and moves focus per the action verb. `aria-disabled="true"` items are
//! skipped via the `:not(:disabled)` selector when paired with HTML `disabled`,
//! and additionally callers can use `aria-disabled` together with the same
//! selector by relying on `[aria-disabled="true"]` not matching `:disabled`
//!, for those cases, the JS is unchanged from the F2 implementation; the
//! mode-tab context menu's per-item `aria-disabled` honoring is handled by
//! its click-handler bail-out, while traversal still rolls past every item.

use dioxus::prelude::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FocusAction {
    Next,
    Prev,
    First,
    Last,
}

impl FocusAction {
    fn as_verb(self) -> &'static str {
        match self {
            Self::Next => "down",
            Self::Prev => "up",
            Self::First => "first",
            Self::Last => "last",
        }
    }
}

/// Inline JS body. Keeps the eval call site a one-liner.
const MENU_FOCUS_JS: &str = r#"
(function(menuId, action) {
    var menu = document.getElementById(menuId);
    if (!menu) return;
    var items = menu.querySelectorAll('[role="menuitem"]:not(:disabled)');
    if (items.length === 0) return;
    var active = document.activeElement;
    var idx = Array.prototype.indexOf.call(items, active);
    var target = null;
    if (action === 'down') {
        target = (idx === -1 || idx === items.length - 1) ? items[0] : items[idx + 1];
    } else if (action === 'up') {
        target = (idx === -1 || idx === 0) ? items[items.length - 1] : items[idx - 1];
    } else if (action === 'first') {
        target = items[0];
    } else if (action === 'last') {
        target = items[items.length - 1];
    }
    if (target) target.focus();
})
"#;

/// Move keyboard focus among `[role=menuitem]` descendants of the element
/// with id=`menu_id`. Skips items whose underlying HTML `disabled` attribute
/// is set (the F2 dropdown items use HTML `disabled`); the F7 context menu
/// uses `aria-disabled` plus a click-handler bail-out, with the focus walker
/// still rolling through them.
///
/// # Safety contract
///
/// `menu_id` is interpolated directly into a `document::eval` JS string
/// (see the format! call below), which means it MUST be JS-string-safe.
/// Callers should derive it from a trusted integer or a static slug -
/// **never from user input** (mode names, profile names, etc). The F7
/// `mode_tabs/context_menu.rs` constructs `menu_id = "mode-tab-menu-{idx}"`
/// from the tab's positional index for exactly this reason.
pub(crate) fn focus_menu_item(menu_id: &str, action: FocusAction) {
    let verb = action.as_verb();
    let _ = document::eval(&format!("{MENU_FOCUS_JS}('{menu_id}', '{verb}');"));
}
