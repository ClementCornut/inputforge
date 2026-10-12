use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Props)]
struct Props {
    context: Rc<RefCell<Option<Context>>>,
    renders: Rc<Cell<usize>>,
}

impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.context, &other.context) && Rc::ptr_eq(&self.renders, &other.renders)
    }
}

fn harness(props: Props) -> Element {
    let Props { context, renders } = props;
    let view = use_signal(View::default);
    *context.borrow_mut() = Some(Context {
        view,
        dispatch: Callback::new(|_| {}),
    });
    renders.set(renders.get() + 1);
    let label = view.read().toggle.map_or("Unavailable", Action::label);
    rsx! { span { "{label}" } }
}

#[test]
fn identical_routing_snapshots_do_not_rerender_subscribers() {
    let context = Rc::new(RefCell::new(None));
    let renders = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new_with_props(
        harness,
        Props {
            context: Rc::clone(&context),
            renders: Rc::clone(&renders),
        },
    );
    dom.rebuild_in_place();
    let routing = context.borrow().unwrap();
    dom.in_runtime(|| routing.publish(View::default()));
    dom.render_immediate_to_vec();
    assert_eq!(renders.get(), 1);
    dom.in_runtime(|| {
        routing.publish(View {
            toggle: Some(Action::Start),
            ..View::default()
        });
    });
    dom.render_immediate_to_vec();
    assert_eq!(renders.get(), 2);
    assert!(dioxus_ssr::render(&dom).contains("Start"));
}
