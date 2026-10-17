use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Field, IntegerInput, IntegerInputError, NumberInput, SegmentedControlOption, TabButton,
    TabsList, TabsRoot, TextInput,
};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
pub(super) struct Log {
    pub numbers: Vec<f64>,
    pub steps: Vec<f64>,
    pub integers: Vec<usize>,
    pub invalid: Vec<IntegerInputError>,
    pub segments: usize,
    pub text_keys: usize,
    pub tabs: Vec<String>,
}
#[derive(Clone, Copy)]
pub(super) struct Values {
    pub number: Signal<f64>,
    pub integer: Signal<usize>,
    pub disabled: Signal<bool>,
    pub show_c: Signal<bool>,
    pub commit_enabled: Signal<bool>,
}
#[derive(Clone, Props)]
struct Props {
    log: Rc<RefCell<Log>>,
    values: Rc<RefCell<Option<Values>>>,
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.log, &other.log)
    }
}
fn root(props: Props) -> Element {
    inputforge_app::ui::use_provider();
    let mut number = use_signal(|| 0.5);
    let integer = use_signal(|| 2);
    let disabled = use_signal(|| false);
    let show_c = use_signal(|| true);
    let commit_enabled = use_signal(|| true);
    let mut tab = use_signal(|| "a".to_owned());
    let text = use_signal(String::new);
    *props.values.borrow_mut() = Some(Values {
        number,
        integer,
        disabled,
        show_c,
        commit_enabled,
    });
    let logs = props.log;
    let n = Rc::clone(&logs);
    let s = Rc::clone(&logs);
    let i = Rc::clone(&logs);
    let inv = Rc::clone(&logs);
    let seg = Rc::clone(&logs);
    let t = Rc::clone(&logs);
    let number_commit = use_callback(move |v| n.borrow_mut().numbers.push(v));
    let integer_commit = use_callback(move |v| i.borrow_mut().integers.push(v));
    rsx! {
        Field { label: "Number".to_owned(), for_id: "number".to_owned(),
            NumberInput { id: "number".to_owned(), value: number, min: 0.0, max: 1.0, step: 0.25, precision: 2, disabled: disabled(),
                oncommit: commit_enabled().then_some(number_commit), onstep: move |v| { s.borrow_mut().steps.push(v); if !commit_enabled() { number.set(v); } } }
        }
        IntegerInput { value: integer, min: 1, max: 10, disabled: disabled(), oncommit: commit_enabled().then_some(integer_commit), oninvalid: move |e| inv.borrow_mut().invalid.push(e) }
        TextInput { value: text, aria_describedby: "text-error".to_owned(), onkeydown: move |_| t.borrow_mut().text_keys += 1 }
        span { id: "text-error", "Error" }
        SegmentedControlOption { value: "one".to_owned(), selected: false, disabled: disabled(), onclick: move |_| seg.borrow_mut().segments += 1, "One" }
        TabsRoot { value: tab, disabled: disabled(), onchange: move |id: String| { logs.borrow_mut().tabs.push(id.clone()); tab.set(id); },
            TabsList { aria_label: "Control tabs".to_owned(),
                TabButton { id: "a".to_owned(), label: "A".to_owned() }
                TabButton { id: "b".to_owned(), label: "B".to_owned(), disabled: true }
                if show_c() { TabButton { id: "c".to_owned(), label: "C".to_owned() } }
            }
        }
    }
}
pub(super) struct Fixture {
    pub dom: VirtualDom,
    pub log: Rc<RefCell<Log>>,
    values: Rc<RefCell<Option<Values>>>,
}
impl Fixture {
    pub(super) fn new() -> Self {
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let log = Rc::new(RefCell::new(Log::default()));
        let values = Rc::new(RefCell::new(None));
        let mut dom = VirtualDom::new_with_props(
            root,
            Props {
                log: Rc::clone(&log),
                values: Rc::clone(&values),
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate_to_vec();
        for index in 1..4 {
            event(&dom, "mounted", index, ());
        }
        dom.render_immediate_to_vec();
        Self { dom, log, values }
    }
    pub(super) fn values(&self) -> Values {
        self.values.borrow().unwrap()
    }
    pub(super) fn render(&mut self) {
        self.dom.render_immediate_to_vec();
        // Dioxus runs synchronization effects after render; publish their text as well.
        self.dom.render_immediate_to_vec();
    }
    pub(super) fn html(&self) -> String {
        dioxus_ssr::render(&self.dom)
    }
}
fn listeners(dom: &VirtualDom, node: &VNode, name: &str, ids: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == name || a.name == format!("on{name}"))
            && let Some(id) = node.mounted_dynamic_attribute(i, dom)
        {
            ids.push(id);
        }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(c) => {
                if let Some(s) = c.mounted_scope(i, node, dom) {
                    listeners(dom, s.root_node(), name, ids);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for n in nodes {
                    listeners(dom, n, name, ids);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn event<T: 'static>(
    dom: &VirtualDom,
    name: &str,
    index: usize,
    data: T,
) -> Event<dyn std::any::Any> {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), name, &mut ids);
    let evt = Event::new(
        Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event(name, evt.clone(), ids[index]);
    evt
}
pub(super) fn form(dom: &VirtualDom, index: usize, text: &str) {
    event(
        dom,
        "input",
        index,
        SerializedFormData::new(text.into(), vec![]),
    );
}
pub(super) fn key(dom: &VirtualDom, index: usize, key: Key) -> Event<dyn std::any::Any> {
    event(
        dom,
        "keydown",
        index,
        SerializedKeyboardData::new(
            key,
            Code::Unidentified,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ),
    )
}
