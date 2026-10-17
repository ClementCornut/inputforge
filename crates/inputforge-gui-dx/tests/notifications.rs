//! The real toast viewport preserves ARIA, interaction and gallery deadline behavior.
#[path = "notifications/events.rs"]
mod events;
use dioxus::prelude::*;
use inputforge_gui_dx::toast::{ToastLevel, ToastQueue, ToastViewport};
use std::{cell::RefCell, rc::Rc, time::Duration};
#[derive(Clone, Props)]
struct Props {
    queue: Rc<RefCell<Option<ToastQueue>>>,
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.queue, &other.queue)
    }
}
fn root(props: Props) -> Element {
    let output = props.queue;
    let queue = inputforge_app::notifications::use_preview();
    *output.borrow_mut() = Some(queue);
    rsx! {ToastViewport{}}
}
async fn drive(dom: &mut VirtualDom, seconds: u64) {
    dom.render_immediate_to_vec();
    tokio::time::advance(Duration::from_secs(seconds)).await;
    dom.render_immediate_to_vec();
}
#[test]
fn viewport_interleaved_hover_focus_escape_close_and_aria_use_real_app_behavior() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let queue = Rc::new(RefCell::new(None));
            let mut dom = VirtualDom::new_with_props(
                root,
                Props {
                    queue: Rc::clone(&queue),
                },
            );
            dom.rebuild_in_place();
            drive(&mut dom, 0).await;
            let q = queue.borrow().unwrap();
            q.push(ToastLevel::Warning, "warning");
            q.push(ToastLevel::Info, "info");
            drive(&mut dom, 0).await;
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("role=\"status\""));
            assert!(html.contains("aria-live=\"polite\""));
            assert_eq!(html.matches("role=\"alert\"").count(), 1);
            assert!(html.find("warning").unwrap() < html.find("info").unwrap());
            drive(&mut dom, 3).await;
            events::send(&dom, "mouseenter", 0, SerializedMouseData::default());
            events::send(&dom, "focusin", 0, SerializedFocusData {});
            events::send(&dom, "mouseleave", 0, SerializedMouseData::default());
            drive(&mut dom, 20).await;
            assert_eq!(q.visible.peek().len(), 1);
            assert_eq!(q.visible.peek()[0].message, "warning");
            events::send(&dom, "focusout", 0, SerializedFocusData {});
            drive(&mut dom, 4).await;
            assert_eq!(q.visible.peek().len(), 1);
            let key = SerializedKeyboardData::new(
                Key::Escape,
                Code::Escape,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            );
            events::send(&dom, "keydown", 0, key);
            drive(&mut dom, 0).await;
            assert_eq!(*q.visible.peek(), []);
            q.push(ToastLevel::Error, "close me");
            drive(&mut dom, 0).await;
            events::send(&dom, "click", 0, SerializedMouseData::default());
            drive(&mut dom, 0).await;
            assert_eq!(*q.visible.peek(), []);
        });
}
#[test]
fn gallery_expiry_restarts_and_teardown_stops_old_owners_before_fresh_mount() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let queue = Rc::new(RefCell::new(None));
            let mut dom = VirtualDom::new_with_props(
                root,
                Props {
                    queue: Rc::clone(&queue),
                },
            );
            dom.rebuild_in_place();
            drive(&mut dom, 0).await;
            let old = queue.borrow().unwrap();
            old.push(ToastLevel::Success, "first");
            drive(&mut dom, 7).await;
            assert_eq!(old.visible.peek().len(), 1);
            old.push(ToastLevel::Success, "first");
            drive(&mut dom, 7).await;
            assert_eq!(old.visible.peek()[0].count, 2);
            drive(&mut dom, 1).await;
            assert_eq!(*old.visible.peek(), []);
            old.push(ToastLevel::Info, "pending at disposal");
            drive(&mut dom, 0).await;
            old.dispose.call(());
            old.push(ToastLevel::Error, "late push");
            old.tick.call((
                Some(vec!["late warning".into()]),
                tokio::time::Instant::now().into_std() + Duration::from_secs(100),
            ));
            assert_eq!(old.visible.peek()[0].message, "pending at disposal");
            drop(dom);
            tokio::time::advance(Duration::from_secs(100)).await;
            let mut fresh = VirtualDom::new_with_props(
                root,
                Props {
                    queue: Rc::clone(&queue),
                },
            );
            fresh.rebuild_in_place();
            drive(&mut fresh, 0).await;
            let new = queue.borrow().unwrap();
            assert_eq!(*new.visible.peek(), []);
            new.push(ToastLevel::Info, "fresh");
            drive(&mut fresh, 8).await;
            assert_eq!(*new.visible.peek(), []);
        });
}
