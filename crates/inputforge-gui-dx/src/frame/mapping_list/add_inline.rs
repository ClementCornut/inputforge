//! Add pad rendering; app callbacks prepare every workflow transition.
use crate::{
    capture::{CaptureFilter, Target},
    components::{
        Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton,
        InputSize, TextInput,
    },
    context::AppContext,
    frame::{mapping_list::source_label, view_state::ViewState},
    icons::Icon as IconKind,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddRequest, AddView, Phase, Scope, Target as RailTarget},
    patterns::live_capture::{CAPTURE_PROMPT, LiveCapture, use_capture},
};
use dioxus::prelude::*;
use inputforge_core::types::InputId;

fn submit(
    mut state: Signal<AddView>,
    mut name: Signal<String>,
    bindings: &Bindings,
    intent: AddIntent,
) -> bool {
    let next = bindings.add.call(AddRequest {
        view: state.peek().clone(),
        intent,
    });
    let capturing = next.phase == Phase::Capturing;
    if next.phase == Phase::Resting {
        name.set(String::new());
    }
    if *state.peek() != next {
        state.set(next);
    }
    capturing
}
fn commit(state: Signal<AddView>, name: Signal<String>, bindings: &Bindings) {
    let raw = name.peek().clone();
    submit(state, name, bindings, AddIntent::Commit(raw));
}
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications"
)]
pub(crate) fn AddInline(
    force_expanded: Signal<bool>,
    pending_duplicate: Signal<Option<RailTarget>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();
    let bindings = use_context::<Bindings>();
    let state = use_signal(AddView::default);
    let mut name = use_signal(String::new);
    let capture = use_capture(
        Target {
            profile: ctx.meta.read().profile_id.clone(),
            mode: view.editing_mode.read().clone(),
            mapping: None,
            name: None,
            actions: None,
            field: "add".into(),
        },
        CaptureFilter::Any,
        move |addr| {
            submit(state, name, &bindings, AddIntent::Captured(addr));
        },
        move |()| {
            submit(state, name, &bindings, AddIntent::CaptureCancelled);
        },
    );
    let ctx_start = ctx.clone();
    let begin = use_callback(move |source: Option<RailTarget>| {
        let scope = Scope {
            profile: ctx_start.meta.peek().profile_id.clone(),
            mode: view.editing_mode.peek().clone(),
        };
        if let Some(source) = &source {
            name.set(format!(
                "{} (copy)",
                source.before.name.as_deref().unwrap_or("(unnamed)")
            ));
        }
        if submit(state, name, &bindings, AddIntent::Start { scope, source }) {
            capture.start.call(());
        }
    });
    let observe = move || {
        let mut pending = pending_duplicate;
        let source = pending.peek().clone();
        if source.is_some() {
            pending.set(None);
            begin.call(source);
        }
        let mut force = force_expanded;
        if *force.peek() {
            force.set(false);
            begin.call(None);
        }
    };
    use_hook(observe);
    use_effect(move || {
        let _ = pending_duplicate.read();
        let _ = force_expanded.read();
        observe();
    });
    use_effect(move || {
        let _ = ctx.config.read();
        let _ = ctx.meta.read();
        let _ = view.editing_mode.read();
        let _ = state.read();
        submit(state, name, &bindings, AddIntent::Reconcile);
        if state.peek().phase == Phase::Resting && *capture.listening.peek() {
            capture.cancel.call(());
        }
    });
    // The existing idle-Escape transport only forwards cancellation to the app.
    let listener = use_hook(|| {
        let key = format!(
            "if-rail-add-{}-{}",
            bindings.instance,
            dioxus::core::current_scope_id().0
        );
        let mut handle = document::eval(&format!(
            r"
            window['{key}']?.();
            const h = (ev) => {{
                if (ev.key === 'Escape' && !ev.target?.closest?.('.if-rail__filter')) dioxus.send('esc');
            }};
            window.addEventListener('keydown', h, true);
            window['{key}'] = () => window.removeEventListener('keydown', h, true);
            (async () => {{ while (await dioxus.recv() !== '__shutdown__') {{}} dioxus.send('shutdown_ack'); }})();
        "
        ));
        spawn(async move {
            while let Ok(message) = handle.recv::<String>().await {
                if message != "esc" {
                    break;
                }
                if !*cap.active.peek() {
                    submit(state, name, &bindings, AddIntent::Cancel);
                }
            }
        });
        (handle, key)
    });
    use_drop(move || {
        let _ = document::eval(&format!(
            "window['{}']?.(); delete window['{}'];",
            listener.1, listener.1
        ));
        let _ = listener.0.send("__shutdown__");
    });
    let error = state.read().error.clone();
    let phase = state.read().phase.clone();
    let body = match phase {
        Phase::Resting => rsx! {
            div { class: "if-add-inline if-add-inline--resting",
                button { r#type: "button", class: "if-add-inline__dashed-row",
                    onclick: move |_| begin.call(None), "aria-label": "Add mapping", "+ Add mapping"
                }
            }
        },
        Phase::Capturing | Phase::Captured(_) => {
            let captured_addr = if let Phase::Captured(addr) = phase {
                Some(addr)
            } else {
                None
            };
            let is_capturing = captured_addr.is_none();
            let cfg = ctx.config.read();
            let (chip_label, device_label, kind_class): (String, String, &'static str) =
                if let Some(addr) = &captured_addr {
                    let (device, input) = source_label::split_label(addr, &cfg);
                    let kind = match addr
                        .input_id()
                        .expect("invariant: add_inline addr always bound (built from F8 capture)")
                    {
                        InputId::Axis { .. } => "axis",
                        InputId::Button { .. } => "button",
                        InputId::Hat { .. } => "hat",
                    };
                    (input, device, kind)
                } else {
                    (String::new(), CAPTURE_PROMPT.to_owned(), "")
                };
            drop(cfg);

            let addr_for_enter = captured_addr.clone();
            let addr_for_btn = captured_addr;
            rsx! {
                div {
                    class: "if-add-inline if-add-inline--pad",
                    onkeydown: move |evt: KeyboardEvent| {
                        // Enter commits when an input has been captured.
                        // Esc is handled by the document-level listener.
                        if evt.key() == Key::Enter
                            && addr_for_enter.is_some()
                        {
                            evt.prevent_default();
                            commit(state, name, &bindings);
                        }
                    },
                    div { class: "if-add-inline__readout",
                        if is_capturing {
                            span { "aria-label": "Listening for input",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    class: "if-add-inline__chip--listening".to_owned(),
                                }
                            }
                        } else {
                            span { "data-kind": "{kind_class}",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    "{chip_label}"
                                }
                            }
                        }
                        span { class: "if-add-inline__device", "{device_label}" }
                        IconButton {
                            icon: IconKind::Refresh,
                            label: "Capture a different input",
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            disabled: is_capturing,
                            onclick: move |_| {
                                if submit(state, name, &bindings, AddIntent::Refresh) { capture.start.call(()); }
                            },
                        }
                    }
                    TextInput {
                        value: ReadSignal::from(name),
                        size: InputSize::Sm,
                        placeholder: "Mapping name".to_owned(),
                        oninput: move |evt: FormEvent| name.set(evt.value()),
                    }
                    div { class: "if-add-inline__actions",
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                                capture.cancel.call(());
                                submit(state, name, &bindings, AddIntent::Cancel);
                            },
                            "Cancel"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            disabled: is_capturing,
                            onclick: move |_| {
                                if addr_for_btn.is_some() {
                                    commit(state, name, &bindings);
                                }
                            },
                            "Add"
                        }
                    }
                }
            }
        }
        Phase::Collision(existing) => {
            let existing_name = existing.name.as_deref().unwrap_or("(unnamed)");
            let cfg = ctx.config.read();
            let captured_label = source_label::format(&existing.input, &cfg);
            drop(cfg);
            rsx! {
                div { class: "if-add-inline if-add-inline--collision",
                    div { class: "if-add-inline__collision-text",
                        Badge { variant: BadgeVariant::Warning, "Collision" }
                        span { class: "if-add-inline__collision-text-body",
                            " {captured_label} already mapped to "
                            strong { "{existing_name}" }
                            "."
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            submit(state, name, &bindings, AddIntent::Open);
                        },
                        "Edit existing"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            submit(state, name, &bindings, AddIntent::Cancel);
                        },
                        "Cancel"
                    }
                }
            }
        }
    };
    rsx! {
        if let Some(error) = error { div { role: "alert", "{error}" } }
        {body}
    }
}
