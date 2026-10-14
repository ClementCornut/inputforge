use dioxus::prelude::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    task::{Context, Poll, Waker},
};
#[derive(Default)]
pub(super) struct Channel {
    incoming: VecDeque<serde_json::Value>,
    sent: Vec<serde_json::Value>,
    wake: Option<Waker>,
}
impl Channel {
    fn push(&mut self, value: serde_json::Value) {
        self.incoming.push_back(value);
        if let Some(wake) = self.wake.take() {
            wake.wake();
        }
    }
}
struct Evaluator(Rc<RefCell<Channel>>);
impl document::Evaluator for Evaluator {
    fn send(&self, data: serde_json::Value) -> Result<(), document::EvalError> {
        let mut c = self.0.borrow_mut();
        c.sent.push(data);
        c.push(serde_json::Value::String("shutdown_ack".into()));
        Ok(())
    }
    fn poll_recv(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        let mut c = self.0.borrow_mut();
        if let Some(value) = c.incoming.pop_front() {
            return Poll::Ready(Ok(value));
        }
        c.wake = Some(context.waker().clone());
        Poll::Pending
    }
    fn poll_join(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        Poll::Pending
    }
}
#[derive(Default)]
pub(super) struct Document {
    owner: Owner<UnsyncStorage>,
    pub listeners: RefCell<Vec<Rc<RefCell<Channel>>>>,
    scripts: RefCell<Vec<String>>,
}
impl Document {
    pub(super) fn pointer_cleanups(&self) -> Vec<String> {
        self.scripts
            .borrow()
            .iter()
            .filter(|script| script.starts_with("if (window.__ifSortableDrag?."))
            .cloned()
            .collect()
    }
    pub(super) fn pointer_installs(&self) -> usize {
        self.scripts
            .borrow()
            .iter()
            .filter(|script| script.contains("__ifSortableStart ??="))
            .count()
    }
    pub(super) fn keyboard(&self, key: &str, alt: u8) {
        let index = self
            .scripts
            .borrow()
            .iter()
            .filter(|s| s.contains("window.addEventListener"))
            .position(|s| s.contains("if-rail-keyboard-"))
            .unwrap();
        self.listeners.borrow()[index]
            .borrow_mut()
            .push(serde_json::json!([key, 0, 0, alt]));
    }
    pub(super) fn installs(&self) -> usize {
        self.listeners.borrow().len()
    }
    pub(super) fn shutdowns(&self) -> usize {
        self.listeners
            .borrow()
            .iter()
            .filter(|c| c.borrow().sent == vec![serde_json::json!("__shutdown__")])
            .count()
    }
    pub(super) fn cleanups(&self) -> usize {
        self.scripts
            .borrow()
            .iter()
            .filter(|s| s.contains("delete window[") && !s.contains("window.addEventListener"))
            .count()
    }
}
impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        let install = js.contains("window.addEventListener");
        self.scripts.borrow_mut().push(js);
        if install {
            let c = Rc::new(RefCell::new(Channel::default()));
            self.listeners.borrow_mut().push(Rc::clone(&c));
            document::Eval::new(self.owner.insert(Box::new(Evaluator(c))))
        } else {
            document::Document::eval(&document::NoOpDocument, String::new())
        }
    }
}
