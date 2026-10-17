//! Parent-owned app callbacks reject obsolete child rendering publications.
use dioxus::prelude::*;
use inputforge_core::{action::Mapping, state::AppState, types::InputAddress};
use inputforge_gui_dx::{capture::Capture, mapping_edits::ui};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
#[derive(Clone)]
struct Context {
    show: Rc<RefCell<Option<Signal<bool>>>>,
    values: Rc<RefCell<Option<Values>>>,
}
#[derive(Clone)]
struct Values {
    header: ui::Header,
    rename: ui::Rename,
    rail: ui::Rail,
    add: ui::Add,
    bindings: ui::Bindings,
}
fn root() -> Element {
    let context = use_context::<Context>();
    let controller = use_hook(|| {
        let (commands, _) = mpsc::channel();
        inputforge_app::mapping_edits::Controller::new(
            Arc::new(RwLock::new(AppState::new())),
            commands,
        )
    });
    use_context_provider(|| controller.bindings().ui);
    let show = use_signal(|| true);
    *context.show.borrow_mut() = Some(show);
    rsx! {if *show.read() {Child {}}}
}
#[component]
fn Child() -> Element {
    let context = use_context::<Context>();
    let bindings = use_context::<ui::Bindings>();
    let value = use_signal(String::new);
    let error = use_signal(|| None);
    let delete = use_signal(|| None);
    let header = ui::Header {
        value,
        error,
        armed: use_signal(|| false),
        draft: use_signal(|| Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![],
        }),
    };
    let rename = ui::Rename {
        value,
        error,
        open: use_signal(|| None),
        target: None,
    };
    let rail = ui::Rail {
        filter: ui::Filter {
            query: value,
            device: use_signal(|| None),
        },
        prepared: use_signal(|| None),
        menu: use_signal(|| None),
        rename: rename.open,
        duplicate: use_signal(|| None),
        delete,
        error,
        live: use_signal(String::new),
    };
    let add = ui::Add {
        state: use_signal(Default::default),
        name: value,
        capture: Capture {
            listening: use_signal(|| false),
            start: Callback::new(|()| {}),
            cancel: Callback::new(|()| {}),
        },
    };
    *context.values.borrow_mut() = Some(Values {
        header,
        rename,
        rail,
        add,
        bindings,
    });
    rsx! {span {}}
}
#[test]
fn disposed_child_callbacks_do_not_publish_or_dispatch() {
    for kind in 0..6 {
        let context = Context {
            show: Rc::default(),
            values: Rc::default(),
        };
        let mut dom = VirtualDom::new(root);
        dom.provide_root_context(context.clone());
        dom.rebuild_in_place();
        let values = context.values.borrow().clone().unwrap();
        dom.in_runtime(|| context.show.borrow().unwrap().set(false));
        dom.render_immediate_to_vec();
        dom.in_runtime(|| match kind {
            0 => values.bindings.header.call(ui::HeaderRequest {
                publication: values.header,
                profile: None,
                target: Mapping {
                    input: InputAddress::Unbound,
                    mode: "Default".into(),
                    name: None,
                    actions: vec![],
                },
                event: ui::HeaderEvent::Input("obsolete".into()),
            }),
            1 => {
                let _ = values.bindings.rename.call((
                    values.rename.clone(),
                    ui::RenameEvent::Input("obsolete".into()),
                ));
            }
            2 => {
                assert!(
                    !values
                        .bindings
                        .rail
                        .call((values.rail, ui::RailEvent::Close))
                );
            }
            3 => values
                .bindings
                .add
                .call((values.add, ui::AddEvent::Input("obsolete".into()))),
            4 => {
                assert!(!values.bindings.delete.call(ui::DeleteRequest {
                    target: values.rail.delete,
                    error: values.rail.error,
                    confirm: false
                }));
            }
            _ => values.bindings.drag.call(ui::DragRequest {
                publication: values.rail.prepared,
                filter: values.rail.filter,
                lookup: inputforge_gui_dx::mapping_rail::Lookup {
                    scope: inputforge_gui_dx::mapping_rail::Scope {
                        profile: None,
                        mode: "Default".into(),
                    },
                    input: InputAddress::Unbound,
                    name: None,
                    filtered: false,
                },
            }),
        });
    }
}
