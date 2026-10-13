use dioxus::prelude::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[derive(Default)]
pub(super) struct Channel {
    incoming: VecDeque<String>,
    pub(super) sent: Vec<String>,
    wake: Option<Waker>,
}
impl Channel {
    pub(super) fn escape(&mut self) {
        self.incoming.push_back("esc".into());
        if let Some(wake) = self.wake.take() {
            wake.wake();
        }
    }
}
struct Evaluator(Rc<RefCell<Channel>>);
impl document::Evaluator for Evaluator {
    fn send(&self, data: serde_json::Value) -> Result<(), document::EvalError> {
        let mut channel = self.0.borrow_mut();
        channel.sent.push(data.as_str().unwrap().into());
        channel.incoming.push_back("shutdown_ack".into());
        if let Some(wake) = channel.wake.take() {
            wake.wake();
        }
        Ok(())
    }
    fn poll_recv(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, document::EvalError>> {
        let mut channel = self.0.borrow_mut();
        if let Some(value) = channel.incoming.pop_front() {
            return Poll::Ready(Ok(value.into()));
        }
        channel.wake = Some(context.waker().clone());
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
    pub(super) listeners: RefCell<Vec<Rc<RefCell<Channel>>>>,
    pub(super) scripts: RefCell<Vec<String>>,
}
impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        let capture = js.contains("if-live-capture-");
        let install = capture && js.contains("window.addEventListener");
        if capture {
            self.scripts.borrow_mut().push(js);
        }
        if install {
            let channel = Rc::new(RefCell::new(Channel::default()));
            self.listeners.borrow_mut().push(Rc::clone(&channel));
            document::Eval::new(self.owner.insert(Box::new(Evaluator(channel))))
        } else {
            document::Document::eval(&document::NoOpDocument, String::new())
        }
    }
}
