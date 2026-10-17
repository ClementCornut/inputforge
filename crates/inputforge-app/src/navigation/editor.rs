//! Scoped editor shortcuts, nested paths and rendering-state invalidation.
use super::{
    Controller,
    editor_keys::{FocusTarget, KbIntent, decide},
    listener,
};
use dioxus::prelude::*;
use inputforge_core::action::ActionBranch;
use inputforge_gui_dx::{
    mapping_edits::{Edit, Request, StageId, StageIdSegment},
    navigation::{EditorPublication, Listener, StageMenu},
};
use std::{cell::Cell, rc::Rc, time::Instant};
type Event = (String, u8, u8, u8, String, String, f64, f64, String, String);
pub(super) fn mount(c: Controller, p: EditorPublication) -> Listener {
    let key = listener::key("editor-keyboard");

    let mut handle = document::eval(
        &include_str!("../../assets/editor_shortcuts.js").replace("__LISTENER_KEY__", &key),
    );
    let closed = Rc::new(Cell::new(false));
    let stop = listener::close(handle, key, Rc::clone(&closed));
    spawn(async move {
        while let Ok(event) = handle.recv::<Event>().await {
            if closed.get() || c.inner.borrow().disposed {
                break;
            }
            route(&c, p, event);
        }
    });
    Listener {
        close: stop,
        query: Callback::new(|_| {}),
        device: Callback::new(|_| {}),
    }
}
fn route(
    c: &Controller,
    mut p: EditorPublication,
    (key, ctrl, shift, alt, focus, path_text, x, y, context, actions): Event,
) {
    let Some(nav) = c.inner.borrow().chrome.publication else {
        return;
    };
    let selection = nav.selected_mapping.peek().clone();
    let Some(state) = c.state.try_read() else {
        return;
    };
    let profile = state.active_profile.as_ref().map(|p| p.id().clone());
    if context != format!("{profile:?}:{:?}", selection.as_ref()) {
        return;
    }
    let Some(mapping_key) = selection else { return };
    let before = c.inner.borrow().history.current(&state, &mapping_key);
    drop(state);
    let focus = match focus.as_str() {
        "Input" => FocusTarget::Input,
        "Stage" => FocusTarget::Stage,
        "Editor" => FocusTarget::Editor,
        _ => FocusTarget::Outside,
    };
    let intent = decide(&key, ctrl != 0, shift != 0, alt != 0, focus);
    if matches!(
        intent,
        KbIntent::StageMoveUp | KbIntent::StageMoveDown | KbIntent::StageMenuOpen
    ) && before
        .as_ref()
        .is_none_or(|m| actions != format!("{:?}", m.actions))
    {
        return;
    }
    match intent {
        KbIntent::Undo => {
            let _ = c.undo(&mapping_key);
        }
        KbIntent::Redo => {
            let _ = c.redo(&mapping_key);
        }
        KbIntent::StageMenuOpen => {
            if let Some(stage) = parse_stage(&path_text)
                && before
                    .as_ref()
                    .is_some_and(|m| super::super::paths::at(&m.actions, &stage.0).is_some())
            {
                p.menu.set(Some(StageMenu { stage, x, y }));
            }
        }
        KbIntent::StageMoveUp | KbIntent::StageMoveDown => {
            let Some(path) = parse_stage(&path_text) else {
                return;
            };
            let Some(before) = before else { return };
            let down = key == "ArrowDown";
            let result = c.edit(
                Request {
                    profile,
                    before,
                    edit: Edit::Step { path, down },
                    label: if down {
                        "Move stage down"
                    } else {
                        "Move stage up"
                    }
                    .into(),
                    group: None,
                    editor: true,
                },
                Instant::now(),
                false,
            );
            p.error.set(result.err());
        }
        KbIntent::PassThrough => {}
    }
}
pub(super) fn parse_stage(raw: &str) -> Option<StageId> {
    let segments = raw
        .split('.')
        .map(|s| match s {
            "T" => Some(StageIdSegment::Branch(ActionBranch::ConditionalTrue)),
            "F" => Some(StageIdSegment::Branch(ActionBranch::ConditionalFalse)),
            "TS" => Some(StageIdSegment::Branch(ActionBranch::TapSingle)),
            "TD" => Some(StageIdSegment::Branch(ActionBranch::TapDouble)),
            "PS" => Some(StageIdSegment::Branch(ActionBranch::PressShort)),
            "PL" => Some(StageIdSegment::Branch(ActionBranch::PressLong)),
            s => s.parse().ok().map(StageIdSegment::Index),
        })
        .collect::<Option<Vec<_>>>()?;
    if segments.is_empty()
        || segments
            .iter()
            .enumerate()
            .any(|(i, s)| matches!(s, StageIdSegment::Index(_)) != (i % 2 == 0))
        || segments.len() % 2 == 0
    {
        return None;
    }
    Some(StageId(segments))
}
