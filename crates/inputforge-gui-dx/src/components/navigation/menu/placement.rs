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

/// Keep a cursor-opened menu within the same padded viewport as trigger menus.
pub(super) fn cursor_placement(
    cursor: MenuAnchor,
    menu: MenuSize,
    viewport: Viewport,
) -> MenuPlacement {
    MenuPlacement {
        left: cursor.x.clamp(
            VIEWPORT_PADDING,
            (viewport.width - menu.width - VIEWPORT_PADDING).max(VIEWPORT_PADDING),
        ),
        top: cursor.y.clamp(
            VIEWPORT_PADDING,
            (viewport.height - menu.height - VIEWPORT_PADDING).max(VIEWPORT_PADDING),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_menu_clamps_both_edges_and_oversized_content() {
        let viewport = Viewport {
            width: 500.0,
            height: 400.0,
        };
        let placement = cursor_placement(
            MenuAnchor { x: 490.0, y: 390.0 },
            MenuSize {
                width: 100.0,
                height: 100.0,
            },
            viewport,
        );
        assert_eq!(
            placement,
            MenuPlacement {
                left: 396.0,
                top: 296.0
            }
        );
        let placement = cursor_placement(
            MenuAnchor { x: -20.0, y: -10.0 },
            MenuSize {
                width: 900.0,
                height: 800.0,
            },
            viewport,
        );
        assert_eq!(
            placement,
            MenuPlacement {
                left: 4.0,
                top: 4.0
            }
        );
    }
}
