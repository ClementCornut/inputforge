//! Notification state, warning delivery and disposal use the real app callbacks.
use dioxus::prelude::*;
use inputforge_gui_dx::toast::{Interaction, ToastLevel, ToastQueue};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
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
    let renders = use_context::<Rc<Cell<usize>>>();
    renders.set(renders.get() + 1);
    let visible = use_signal(Vec::new);
    let bindings = use_hook(inputforge_app::notifications::bindings);
    let queue = use_hook(|| bindings.mount.call(visible));
    use_drop(move || queue.dispose.call(()));
    *output.borrow_mut() = Some(queue);
    rsx! { for toast in visible.read().iter() { p { "{toast.message} ×{toast.count}" } } }
}
fn with_queue(run: impl FnOnce(ToastQueue, &mut VirtualDom)) {
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
            dom.provide_root_context(Rc::new(Cell::new(0usize)));
            dom.rebuild_in_place();
            let callbacks = queue.borrow().unwrap();
            run(callbacks, &mut dom);
        });
}
fn tick(queue: ToastQueue, warnings: Option<Vec<&str>>) {
    queue.tick.call((
        warnings.map(|xs| xs.into_iter().map(str::to_owned).collect()),
        tokio::time::Instant::now().into_std(),
    ));
}
#[test]
fn expiration_does_not_resurrect_duplicate_and_latest_only_keeps_order() {
    with_queue(|q, _| {
        q.push(ToastLevel::Info, "alpha");
        q.push(ToastLevel::Info, "beta");
        q.push(ToastLevel::Info, "alpha");
        q.push(ToastLevel::Info, "alpha");
        let rows = q.visible.peek();
        assert_eq!(
            rows.iter()
                .map(|r| (&*r.message, r.count))
                .collect::<Vec<_>>(),
            vec![("alpha", 1), ("beta", 1), ("alpha", 2)]
        );
        let old_id = rows[2].id;
        drop(rows);
        q.tick.call((
            None,
            tokio::time::Instant::now().into_std() + Duration::from_secs(8),
        ));
        assert_eq!(*q.visible.peek(), []);
        q.push(ToastLevel::Info, "alpha");
        assert_eq!(q.visible.peek()[0].count, 1);
        assert_ne!(q.visible.peek()[0].id, old_id);
    });
}
#[test]
fn warning_reset_same_length_replacement_and_repeated_frames_do_not_replay() {
    with_queue(|q, _| {
        tick(q, Some(vec!["historic"]));
        assert_eq!(*q.visible.peek(), []);
        tick(q, Some(vec!["historic", "disk failed"]));
        tick(q, Some(vec!["historic", "disk failed"]));
        assert_eq!(q.visible.peek().len(), 1);
        assert_eq!(q.visible.peek()[0].count, 1);
        tick(q, Some(vec!["disk failed"]));
        assert_eq!(q.visible.peek().len(), 1);
        tick(q, Some(vec!["permission denied"]));
        assert_eq!(q.visible.peek().len(), 2);
        tick(q, Some(vec![]));
        tick(q, Some(vec!["disk failed"]));
        assert_eq!(q.visible.peek().len(), 3);
    });
}
#[test]
fn fifo_cap_and_stale_dismiss_never_change_another_entry() {
    with_queue(|q, _| {
        q.push(ToastLevel::Success, "first");
        let stale = q.visible.peek()[0].id;
        for i in 1..=5 {
            q.push(ToastLevel::Info, format!("row-{i}"));
        }
        q.event.call((stale, Interaction::Dismiss));
        assert_eq!(
            q.visible
                .peek()
                .iter()
                .map(|r| r.message.clone())
                .collect::<Vec<_>>(),
            ["row-1", "row-2", "row-3", "row-4", "row-5"]
        );
        let id = q.visible.peek()[0].id;
        q.event.call((id, Interaction::Key(Key::Enter)));
        assert_eq!(q.visible.peek().len(), 5);
        q.event.call((id, Interaction::Key(Key::Escape)));
        q.event.call((id, Interaction::Dismiss));
        assert_eq!(q.visible.peek().len(), 4);
    });
}

#[test]
fn unchanged_ticks_and_pause_events_do_not_publish_equal_rows() {
    with_queue(|q, dom| {
        q.push(ToastLevel::Info, "stable");
        dom.render_immediate_to_vec();
        let renders = dom.in_scope(ScopeId::ROOT, use_context::<Rc<Cell<usize>>>);
        let before = renders.get();
        tick(q, Some(vec![]));
        q.event
            .call((q.visible.peek()[0].id, Interaction::Hover(true)));
        q.event
            .call((q.visible.peek()[0].id, Interaction::Hover(false)));
        dom.render_immediate_to_vec();
        assert_eq!(
            renders.get(),
            before,
            "equal rows must not wake subscribers"
        );
    });
}

#[path = "notifications/backend.rs"]
mod backend;
