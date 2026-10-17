//! Viewport placement geometry for rendered floating lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Horizontal alignment of a floating menu relative to its trigger.
pub enum Anchor {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Cursor anchor in page-space CSS pixels.
pub struct MenuAnchor {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MenuPlacement {
    pub(super) left: f64,
    pub(super) top: f64,
}

// Match the existing four-pixel trigger gap without placing chrome flush to the viewport.
const VIEWPORT_PADDING: f64 = 4.0;

// Preserve the original dropdown spacing above or below its trigger.
const TRIGGER_GAP: f64 = 4.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct TriggerRect {
    pub(super) left: f64,
    pub(super) top: f64,
    pub(super) right: f64,
    pub(super) bottom: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MenuSize {
    pub(super) width: f64,
    pub(super) height: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Viewport {
    pub(super) width: f64,
    pub(super) height: f64,
}

pub(super) fn compute_placement(
    trigger: TriggerRect,
    menu: MenuSize,
    viewport: Viewport,
    anchor: Anchor,
) -> MenuPlacement {
    let desired_left = match anchor {
        Anchor::Start => trigger.left,
        Anchor::Center => f64::midpoint(trigger.left, trigger.right) - menu.width / 2.0,
        Anchor::End => trigger.right - menu.width,
    };
    let max_left = (viewport.width - menu.width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
    let left = desired_left.clamp(VIEWPORT_PADDING, max_left);

    let below_top = trigger.bottom + TRIGGER_GAP;
    let above_top = trigger.top - menu.height - TRIGGER_GAP;
    let fits_below = below_top + menu.height <= viewport.height - VIEWPORT_PADDING;
    let fits_above = above_top >= VIEWPORT_PADDING;
    let top = if fits_below {
        below_top
    } else if fits_above {
        above_top
    } else {
        let space_below = viewport.height - trigger.bottom;
        let space_above = trigger.top;
        if space_below >= space_above {
            (viewport.height - menu.height - VIEWPORT_PADDING).max(VIEWPORT_PADDING)
        } else {
            VIEWPORT_PADDING
        }
    };

    MenuPlacement { left, top }
}
