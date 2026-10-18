//! Raw rendering publication for the application-owned shortcut listener.
use dioxus::prelude::*;
pub(crate) fn use_kb_listener() {
    let editor = use_context::<super::EditorState>();
    let bindings = use_context::<crate::mapping_edits::Bindings>();
    let mut listener = use_signal(|| None::<crate::navigation::Listener>);
    use_effect(move || {
        if listener.peek().is_none() {
            listener.set(Some(bindings.navigation.editor.call(editor.publication())));
        }
    });
    use_drop(move || {
        if let Some(l) = listener.peek().as_ref() {
            l.close.call(());
        }
    });
}

#[cfg(test)]
#[path = "keyboard_tests.rs"]
mod tests;
