//! Fake eval transport; executable JS coverage lives in the app listener suite.
use dioxus::prelude::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    task::{Context, Poll, Waker},
};
#[derive(Default)]
struct Channel {
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
        c.push(serde_json::json!("shutdown_ack"));
        Ok(())
    }
    fn poll_recv(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        let mut c = self.0.borrow_mut();
        if let Some(value) = c.incoming.pop_front() {
            Poll::Ready(Ok(value))
        } else {
            c.wake = Some(context.waker().clone());
            Poll::Pending
        }
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
    listeners: RefCell<Vec<Rc<RefCell<Channel>>>>,
    scripts: RefCell<Vec<String>>,
}
impl Document {
    pub(super) fn pointer(&self, listener: usize, kind: &str, x: f64, y: f64) {
        self.listeners.borrow()[listener]
            .borrow_mut()
            .push(serde_json::json!({"kind":kind,"x":x,"y":y,"rl":0,"rt":0,"rs":200}));
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
            .filter(|s| s.starts_with("window.__inputforgeInstruments?.get("))
            .count()
    }
}
impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        let install = js.contains("const plotId =") && js.contains("__inputforgeInstruments");
        self.scripts.borrow_mut().push(js);
        if install {
            let channel = Rc::new(RefCell::new(Channel::default()));
            self.listeners.borrow_mut().push(Rc::clone(&channel));
            document::Eval::new(self.owner.insert(Box::new(Evaluator(channel))))
        } else {
            document::Document::eval(&document::NoOpDocument, String::new())
        }
    }
}
