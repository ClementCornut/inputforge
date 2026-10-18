//! Signal publication and supplied callbacks for mounted instrument rendering.
use super::{bridge::stage_id_dom_id, geometry};
use crate::{
    context::AppContext,
    frame::{MappingKey, mapping_editor::EditorState},
    instruments::{Event, Mount, Open, View},
    mapping_edits::{Bindings, StageId},
    mapping_rail::{Scope, Target},
};
use dioxus::prelude::*;
use inputforge_core::action::{Action, Mapping};
pub(crate) fn use_instrument(
    key: &MappingKey,
    path: &StageId,
    initial_value: Action,
) -> (
    Signal<View>,
    EventHandler<Event>,
    EventHandler<MountedEvent>,
    String,
) {
    let ctx = use_context::<AppContext>();
    let bindings = use_context::<Bindings>().instruments;
    let editor = use_context::<EditorState>();
    let initial = Open {
        target: ctx
            .config
            .peek()
            .mappings
            .iter()
            .find(|m| m.input == key.1 && m.mode == key.0)
            .and_then(|m| {
                Some(Target {
                    scope: Scope {
                        profile: ctx.meta.peek().profile_id.clone(),
                        mode: key.0.clone(),
                    },
                    before: Mapping {
                        input: key.1.clone(),
                        mode: key.0.clone(),
                        name: m.name.clone(),
                        actions: ctx.config.peek().selected_mapping_actions.clone()?,
                    },
                })
            }),
        path: path.clone(),
        seed: initial_value,
    };
    let mut view = use_signal(move || bindings.open.call(initial));
    let id = view.peek().id;
    let error_path = path.clone();
    let publish = use_callback(move |next: View| {
        if view.try_peek().is_err() {
            return;
        }
        let mut error = editor.error;
        error.set(next.error.clone());
        let mut hints = editor.malformed_hints;
        if let Some(error) = &next.error {
            hints.write().insert(error_path.clone(), error.clone());
        } else {
            hints.write().remove(&error_path);
        }
        if *view.peek() != next {
            view.set(next);
        }
    });
    let send = use_callback(move |event| {
        if let Some(next) = bindings.event.call((id, event)) {
            publish.call(next);
        }
    });
    use_effect(move || {
        let _config = ctx.config.read();
        let _meta = ctx.meta.read();
        let _history = editor.undo_log.read();
        send.call(Event::Refresh);
    });
    use_drop(move || bindings.close.call(id));
    let plot = format!("{}-{id}", stage_id_dom_id("if-instrument-plot", path));
    let plot_for_mount = plot.clone();
    let observe = use_callback(move |pointer| geometry::observe(&pointer, &view.peek().value));
    let mounted = use_callback(move |_: MountedEvent| {
        bindings.mount.call(Mount {
            id,
            plot: plot_for_mount.clone(),
            observe,
            publish,
        });
    });
    (view, send, mounted, plot)
}
