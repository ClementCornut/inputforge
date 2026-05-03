// Rust guideline compliant 2026-05-03

//! Document-level mouse-event bridge for instrument plots. Dioxus 0.7 desktop
//! does not deliver mousedown/move/up/dblclick/contextmenu to non-button
//! divs, so we install raw addEventListener handlers via `document::eval`,
//! parse the JSON payload back through the eval channel, and route each event
//! to the per-instrument dispatch closure provided by the caller.

use std::fmt::Write as _;

use dioxus::prelude::*;
use serde::Deserialize;

use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

/// JavaScript installed by `mount_mouse_bridge` via `document::eval`. Captures
/// mouse events at the document level (where they fire reliably in `WebView2`)
/// and forwards them to Rust as `BridgeEvent` JSON payloads. Coords are coerced
/// to integers (`| 0`) to sidestep the float-vs-integer deserialization bug
/// tracked in Dioxus issue #4706.
///
/// `__PLOT_ID__` is replaced with the wrapper div's stage-id-derived DOM id so
/// each instrument's listeners scope to its own plot rect. A `dragging` flag
/// keeps move/up firing when the cursor leaves the plot mid-drag (otherwise
/// the user could lose the drag by exiting the plot bounds).
///
/// Listener-scoping rule: events that fire only inside the plot
/// (`mousedown`, `dblclick`, `contextmenu`) attach to `plotEl`, so the browser
/// auto-collects them when the wrapper div is removed from the DOM on
/// component unmount. `mousemove` and `mouseup` MUST stay on `document` because
/// the user can drag past the plot bounds; those two carry an explicit
/// `getElementById(...)` null-check so a stale closure self-disables once its
/// target element is gone. Without that guard, listeners would accumulate
/// across mapping switches / stage collapse cycles and every doc-level mouse
/// event would fan out to N stale closures.
pub(crate) const BRIDGE_JS_TEMPLATE: &str = r"
    var plotEl = document.getElementById('__PLOT_ID__');
    if (!plotEl) return;
    var plotId = '__PLOT_ID__';
    var dragging = false;

    // Re-read getBoundingClientRect on EVERY event. Caching the mount-time
    // rect breaks as soon as the page scrolls, the toolbar layout shifts, or
    // the window resizes; the viewBox coords Rust computes would be stale,
    // and clicks would map to whatever offset the rect drifted to (the
    // symptom: 'I can only place points in the bottom-left quarter'). The
    // live rect (rl, rt, rs) is sent with every payload so the per-instrument
    // dispatcher builds a fresh PlotRect for each handler invocation.
    var sendEvt = function(kind, e) {
        var r = plotEl.getBoundingClientRect();
        dioxus.send({
            kind: kind,
            x: e.clientX | 0,
            y: e.clientY | 0,
            rl: r.left,
            rt: r.top,
            rs: Math.min(r.width, r.height),
        });
    };

    var inPlot = function(e) {
        var r = plotEl.getBoundingClientRect();
        return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    };

    plotEl.addEventListener('mousedown', function(e) {
        if (e.button !== 0) return;
        dragging = true;
        sendEvt('down', e);
    });

    document.addEventListener('mousemove', function(e) {
        if (!document.getElementById(plotId)) return;
        if (!dragging && !inPlot(e)) return;
        sendEvt('move', e);
    });

    document.addEventListener('mouseup', function(e) {
        if (!document.getElementById(plotId)) return;
        if (e.button !== 0) return;
        if (!dragging) return;
        dragging = false;
        sendEvt('up', e);
    });

    plotEl.addEventListener('dblclick', function(e) {
        sendEvt('dbl', e);
    });

    plotEl.addEventListener('contextmenu', function(e) {
        e.preventDefault();
        // Right-click during a left-drag must not leave `dragging` stuck true.
        // Synthesize an 'up' so the dispatcher runs its pointer-up path
        // (commits or reverts the drag); then deliver the 'ctx' for the
        // remove-anchor path.
        if (dragging) {
            dragging = false;
            sendEvt('up', e);
        }
        sendEvt('ctx', e);
    });
";

/// Wire payload for the JS bridge. `x`/`y` are cursor viewport-CSS-pixel
/// coords; `rl`/`rt`/`rs` are the plot wrapper's live `getBoundingClientRect`
/// (left, top, smaller-of-width/height). Sending the live rect with every
/// event prevents stale-rect drift from misprojecting the cursor when the page
/// scrolls or the surrounding layout shifts. Defaults keep deserialization
/// permissive so a malformed message never crashes the dispatcher loop.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BridgeEvent {
    pub kind: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub rl: f64,
    #[serde(default)]
    pub rt: f64,
    #[serde(default)]
    pub rs: f64,
}

/// Build a stable DOM id for an instrument plot wrapper.
///
/// `MountedData::get_client_rect()` does not return a working rect on the
/// Dioxus 0.7 desktop target, so the bridge calls `document::eval` with
/// `getBoundingClientRect()` and the wrapper div needs a unique id selectable
/// from JS. The id is derived from `stage_id` so each instrument body on screen
/// queries its own rect even when multiple stages of the same kind are mounted
/// (e.g. a top-level curve plus one inside a Conditional branch). `prefix`
/// scopes the id namespace per-instrument (`if-curve-plot`, `if-deadzone-plot`,
/// etc.) so distinct instruments cannot collide on the same stage path.
pub(crate) fn stage_id_dom_id(prefix: &str, stage_id: &StageId) -> String {
    let mut s = String::from(prefix);
    for seg in &stage_id.0 {
        match seg {
            StageIdSegment::Index(n) => {
                let _ = write!(s, "-i{n}");
            }
            StageIdSegment::IfTrue => s.push_str("-t"),
            StageIdSegment::IfFalse => s.push_str("-f"),
        }
    }
    s
}

/// Mount the JS bridge for a plot identified by `plot_dom_id`. Returns an
/// `EventHandler<MountedEvent>` to attach via `onmounted: ...`. Each parsed
/// `BridgeEvent` is forwarded to `dispatch_fn`; the spawned task self-exits
/// when the eval channel closes (component unmount).
pub(crate) fn mount_mouse_bridge(
    plot_dom_id: String,
    dispatch_fn: impl Fn(BridgeEvent) + Clone + 'static,
) -> EventHandler<MountedEvent> {
    EventHandler::new(move |_evt: MountedEvent| {
        let id = plot_dom_id.clone();
        let dispatch_fn = dispatch_fn.clone();
        spawn(async move {
            let js = BRIDGE_JS_TEMPLATE.replace("__PLOT_ID__", &id);
            let mut handle = document::eval(&js);
            loop {
                let Ok(payload) = handle.recv::<BridgeEvent>().await else {
                    break;
                };
                dispatch_fn(payload);
            }
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dom_id_for_top_level_stage() {
        let id = StageId(vec![StageIdSegment::Index(2)]);
        assert_eq!(stage_id_dom_id("if-curve-plot", &id), "if-curve-plot-i2");
    }

    #[test]
    fn dom_id_for_nested_stage() {
        let id = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(1),
        ]);
        assert_eq!(
            stage_id_dom_id("if-deadzone-plot", &id),
            "if-deadzone-plot-i0-t-i1"
        );
    }

    #[test]
    fn template_has_placeholder() {
        assert!(BRIDGE_JS_TEMPLATE.contains("__PLOT_ID__"));
    }

    #[test]
    fn parses_event_payload() {
        let raw = r#"{"kind":"down","x":120,"y":80,"rl":10,"rt":20,"rs":300}"#;
        let evt: BridgeEvent = serde_json::from_str(raw).unwrap();
        assert_eq!(evt.kind, "down");
        // Bit-exact comparison: integer-valued f64s round-trip from JSON without
        // rounding, so `to_bits` gives the same canonical representation as the
        // literal and avoids clippy::float_cmp without adding tolerance noise.
        assert_eq!(evt.x.to_bits(), 120.0_f64.to_bits());
        assert_eq!(evt.rs.to_bits(), 300.0_f64.to_bits());
    }
}
