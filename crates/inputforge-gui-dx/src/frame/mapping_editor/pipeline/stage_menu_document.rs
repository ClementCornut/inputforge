//! Deterministic menu measurement for rendered stage-menu placement tests.
use dioxus::{
    core::{DynamicNode, ElementId, TemplateAttribute, TemplateNode},
    prelude::*,
};
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Default)]
pub(super) struct MeasuredDocument {
    owner: Owner<UnsyncStorage>,
}

struct Measurement;
impl document::Evaluator for Measurement {
    fn send(&self, _: serde_json::Value) -> Result<(), document::EvalError> {
        Ok(())
    }

    fn poll_recv(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        Poll::Ready(Ok(serde_json::json!([200.0, 100.0, 1000.0, 800.0])))
    }

    fn poll_join(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        Poll::Pending
    }
}

impl document::Document for MeasuredDocument {
    fn eval(&self, js: String) -> document::Eval {
        if js.contains("r.width, r.height, window.innerWidth, window.innerHeight") {
            document::Eval::new(self.owner.insert(Box::new(Measurement)))
        } else {
            document::Document::eval(&document::NoOpDocument, js)
        }
    }
}

fn mounted_menu(dom: &VirtualDom, node: &VNode) -> Option<ElementId> {
    for (index, attributes) in node.dynamic_attrs.iter().enumerate() {
        if !attributes
            .iter()
            .any(|attribute| matches!(attribute.name, "mounted" | "onmounted"))
        {
            continue;
        }
        let path = node.template.attr_paths[index];
        let mut native = &node.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            if let TemplateNode::Element { children, .. } = native {
                native = &children[usize::from(*child)];
            }
        }
        if matches!(native, TemplateNode::Element { attrs, .. }
            if attrs.iter().any(|attribute| matches!(attribute,
                TemplateAttribute::Static { name: "role", value: "menu", .. })))
        {
            return node.mounted_dynamic_attribute(index, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(index, child)| match child {
            DynamicNode::Component(component) => component
                .mounted_scope(index, node, dom)
                .and_then(|scope| mounted_menu(dom, scope.root_node())),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|child| mounted_menu(dom, child)),
            _ => None,
        })
}

pub(super) fn mount_menu(dom: &VirtualDom) {
    let id = mounted_menu(dom, dom.base_scope().root_node()).expect("native cursor menu mount");
    dom.runtime().handle_event(
        "mounted",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(MountedData::new(()))))
                as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
}
