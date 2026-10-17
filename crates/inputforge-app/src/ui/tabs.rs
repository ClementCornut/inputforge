//! Tab activation, disabled traversal and acknowledged focus selection.
use dioxus::prelude::*;
use inputforge_gui_dx::{
    components::TabsOrientation,
    controls::{TabFocus, TabState, TabsProps, TabsView},
};
pub(super) fn use_tabs(props: TabsProps) -> TabsView {
    let mut pending = use_signal(|| None::<String>);
    let key = use_callback(
        move |(event, entries, orientation): (KeyboardEvent, Vec<TabState>, TabsOrientation)| {
            if (props.disabled)() {
                return;
            }
            let Some(target) = target(&entries, &(props.value)(), &event.key(), orientation) else {
                return;
            };
            event.prevent_default();
            props.onchange.call(target.clone());
            pending.set(Some(target));
        },
    );
    let click = use_callback(move |(id, disabled): (String, bool)| {
        if !disabled && !(props.disabled)() {
            pending.set(None);
            props.onchange.call(id);
        }
    });
    let focus = use_callback(move |(request, entries): (Option<String>, Vec<TabState>)| {
        if (props.disabled)() {
            if pending.peek().is_some() {
                pending.set(None);
            }
            return TabFocus {
                target: None,
                consumed: request.is_some(),
            };
        }
        if let Some(id) = request {
            let entry = entries.iter().find(|entry| entry.id == id);
            if entry.is_some() {
                pending.set(None);
            }
            return TabFocus {
                target: entry.filter(|e| !e.disabled).map(|e| e.id.clone()),
                consumed: entry.is_some(),
            };
        }
        let Some(id) = pending.peek().clone() else {
            return TabFocus::default();
        };
        let entry = entries.iter().find(|e| e.id == id && !e.disabled);
        if entry.is_none() {
            pending.set(None);
            return TabFocus::default();
        }
        if (props.value)() != id {
            return TabFocus::default();
        }
        pending.set(None);
        TabFocus {
            target: Some(id),
            consumed: false,
        }
    });
    TabsView {
        pending: pending.into(),
        key,
        click,
        focus,
    }
}
fn target(
    entries: &[TabState],
    current: &str,
    key: &Key,
    orientation: TabsOrientation,
) -> Option<String> {
    let len = entries.len();
    if len == 0 {
        return None;
    }
    let current = entries.iter().position(|e| e.id == current).unwrap_or(0);
    let (mut index, forward) = match key {
        Key::ArrowRight if orientation == TabsOrientation::Horizontal => {
            ((current + 1) % len, true)
        }
        Key::ArrowLeft if orientation == TabsOrientation::Horizontal => {
            ((current + len - 1) % len, false)
        }
        Key::ArrowDown if orientation == TabsOrientation::Vertical => ((current + 1) % len, true),
        Key::ArrowUp if orientation == TabsOrientation::Vertical => {
            ((current + len - 1) % len, false)
        }
        Key::Home => (0, true),
        Key::End => (len - 1, false),
        _ => return None,
    };
    for _ in 0..len {
        if !entries[index].disabled {
            return Some(entries[index].id.clone());
        }
        index = if forward {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        };
    }
    None
}
