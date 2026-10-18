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
    pub text_inputs: Vec<String>,
    pub tabs: Vec<String>,
}
#[derive(Clone, Copy)]
pub(super) struct Values {
    pub number: Signal<f64>,
    pub integer: Signal<usize>,
    pub disabled: Signal<bool>,
    pub show_c: Signal<bool>,
    pub commit_enabled: Signal<bool>,
    pub text: Signal<String>,
    pub acknowledge_text: Signal<bool>,
    pub other_text_context: Signal<bool>,
}
#[derive(Clone, Props)]
struct Props {
    log: Rc<RefCell<Log>>,
    values: Rc<RefCell<Option<Values>>>,
    scripts: Rc<RefCell<Vec<String>>>,
}
struct Document(Rc<RefCell<Vec<String>>>);
impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.log, &other.log)
    }
}
fn root(props: Props) -> Element {
    use_context_provider(|| {
        Rc::new(Document(Rc::clone(&props.scripts))) as Rc<dyn document::Document>
    });
    inputforge_app::ui::use_provider();
    let mut number = use_signal(|| 0.5);
    let integer = use_signal(|| 2);
    let disabled = use_signal(|| false);
    let show_c = use_signal(|| true);
    let commit_enabled = use_signal(|| true);
    let mut tab = use_signal(|| "a".to_owned());
    let mut text = use_signal(String::new);
    let acknowledge_text = use_signal(|| true);
    let other_text_context = use_signal(|| false);
    let other_text = use_signal(|| "Next profile".to_owned());
    *props.values.borrow_mut() = Some(Values {
        number,
        integer,
        disabled,
        show_c,
        commit_enabled,
        text,
        acknowledge_text,
        other_text_context,
    });
    let logs = props.log;
    let n = Rc::clone(&logs);
    let s = Rc::clone(&logs);
    let i = Rc::clone(&logs);
    let inv = Rc::clone(&logs);
    let seg = Rc::clone(&logs);
    let t = Rc::clone(&logs);
    let text_log = Rc::clone(&logs);
    let number_commit = use_callback(move |v| n.borrow_mut().numbers.push(v));
    let integer_commit = use_callback(move |v| i.borrow_mut().integers.push(v));
    rsx! {
        Field { label: "Number".to_owned(), for_id: "number".to_owned(),
            NumberInput { id: "number".to_owned(), value: number, min: 0.0, max: 1.0, step: 0.25, precision: 2, disabled: disabled(),
                oncommit: commit_enabled().then_some(number_commit), onstep: move |v| { s.borrow_mut().steps.push(v); if !commit_enabled() { number.set(v); } } }
        }
        IntegerInput { id: "integer", aria_label: "Integer", value: integer, min: 1, max: 10, disabled: disabled(), oncommit: commit_enabled().then_some(integer_commit), oninvalid: move |e| inv.borrow_mut().invalid.push(e) }
        TextInput { id: "text", aria_label: "Text", value: if other_text_context() { other_text } else { text }, aria_describedby: "text-error".to_owned(), oninput: move |event: FormEvent| { text_log.borrow_mut().text_inputs.push(event.value()); if acknowledge_text() { text.set(event.value()); } }, onkeydown: move |_| t.borrow_mut().text_keys += 1 }
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
    scripts: Rc<RefCell<Vec<String>>>,
}
impl Fixture {
    pub(super) fn new() -> Self {
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let log = Rc::new(RefCell::new(Log::default()));
        let values = Rc::new(RefCell::new(None));
        let scripts = Rc::new(RefCell::new(Vec::new()));
        let mut dom = VirtualDom::new_with_props(
            root,
            Props {
                log: Rc::clone(&log),
                values: Rc::clone(&values),
                scripts: Rc::clone(&scripts),
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate_to_vec();
        for id in ["tab-a", "tab-b", "tab-c"] {
            send(
                &dom,
                "mounted",
                named_control(&dom, dom.base_scope().root_node(), id).expect("mounted tab button"),
                (),
            );
        }
        dom.render_immediate_to_vec();
        Self {
            dom,
            log,
            values,
            scripts,
        }
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
    pub(super) fn text_resets(&self) -> Vec<String> {
        self.scripts
            .borrow()
            .iter()
            .filter(|s| s.contains("data-if-text-owner"))
            .cloned()
            .collect()
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
fn named_control(dom: &VirtualDom, node: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::{AttributeValue, DynamicNode};
    for (index, attributes) in node.dynamic_attrs.iter().enumerate() {
        if attributes.iter().any(|attribute| {
            matches!(attribute.name, "id" | "aria-label")
                && matches!(&attribute.value, AttributeValue::Text(value) if value == name)
        }) && let Some(id) = node.mounted_dynamic_attribute(index, dom)
        {
            return Some(id);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(index, child)| match child {
            DynamicNode::Component(component) => component
                .mounted_scope(index, node, dom)
                .and_then(|scope| named_control(dom, scope.root_node(), name)),
            DynamicNode::Fragment(nodes) => nodes
                .iter()
                .find_map(|child| named_control(dom, child, name)),
            _ => None,
        })
}

fn send<T: 'static>(
    dom: &VirtualDom,
    name: &str,
    target: dioxus::core::ElementId,
    data: T,
) -> Event<dyn std::any::Any> {
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event(name, event.clone(), target);
    event
}

pub(super) fn event<T: 'static>(
    dom: &VirtualDom,
    name: &str,
    index: usize,
    data: T,
) -> Event<dyn std::any::Any> {
    let target = if matches!(name, "input" | "blur" | "focus" | "keydown") {
        let control = ["number", "integer", "text", "Control tabs"][index];
        named_control(dom, dom.base_scope().root_node(), control).expect("named fixture control")
    } else {
        let mut ids = vec![];
        listeners(dom, dom.base_scope().root_node(), name, &mut ids);
        ids[index]
    };
    send(dom, name, target, data)
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
