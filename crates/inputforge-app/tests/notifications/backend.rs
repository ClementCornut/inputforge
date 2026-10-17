//! Delivered settings requests can fail in core; the warning and inline error remain honest.
use super::*;
use inputforge_core::{
    device::MockInputSource,
    engine::Engine,
    output::{MockKeyboardSink, MockMouseSink, MockOutputSink},
    settings::AppSettings,
    state::AppState,
};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};
type Output = Rc<
    RefCell<
        Option<(
            ToastQueue,
            inputforge_app::settings::Controller,
            Signal<inputforge_gui_dx::settings::View>,
            u64,
        )>,
    >,
>;
#[test]
fn delivered_settings_failure_reaches_notifications_without_expiring_inline_error() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let state = Arc::new(RwLock::new(AppState::new()));
            let (commands, rx) = mpsc::channel();
            let dir = tempfile::tempdir().unwrap();
            let mut engine = Engine::new(
                Box::new(MockInputSource::default()),
                Box::new(MockOutputSink::new()),
                Box::new(MockKeyboardSink::new()),
                Box::new(MockMouseSink::new()),
                Arc::clone(&state),
                rx,
                AppSettings::default(),
                dir.path().join("settings\0.toml"),
                Box::new(inputforge_autostart::mock::MockAutostart::new()),
            );
            let mut dom = VirtualDom::new(|| {
                let queue = inputforge_app::notifications::use_provider();
                let shared = use_context::<Arc<RwLock<AppState>>>();
                let commands =
                    use_context::<mpsc::Sender<inputforge_core::engine::EngineCommand>>();
                let controller =
                    use_hook(|| inputforge_app::settings::Controller::new(shared, commands));
                let view = use_signal(inputforge_gui_dx::settings::View::default);
                let owner = use_hook(|| controller.mount(view));
                *use_context::<Output>().borrow_mut() = Some((queue, controller, view, owner));
                rsx! {div{}}
            });
            let output: Output = Rc::default();
            dom.provide_root_context(Arc::clone(&state));
            dom.provide_root_context(commands);
            dom.provide_root_context(Rc::clone(&output));
            dom.rebuild_in_place();
            let (q, controller, view, owner) = output.borrow().clone().unwrap();
            q.tick.call((
                Some(state.read().warnings.clone()),
                tokio::time::Instant::now().into_std(),
            ));
            dom.in_scope(ScopeId::ROOT, || {
                assert!(controller.dispatch(owner, inputforge_gui_dx::settings::Intent::Skip));
            });
            assert_eq!(*q.visible.peek(), []);
            engine.tick().unwrap();
            dom.in_scope(ScopeId::ROOT, || controller.refresh());
            assert!(view.peek().error.as_ref().unwrap().contains("not saved"));
            q.tick.call((
                Some(state.read().warnings.clone()),
                tokio::time::Instant::now().into_std(),
            ));
            assert_eq!(q.visible.peek().len(), 1);
            assert!(
                q.visible.peek()[0]
                    .message
                    .contains("Could not save settings")
            );
            assert_eq!(q.visible.peek()[0].level, ToastLevel::Warning);
            q.tick.call((
                Some(state.read().warnings.clone()),
                tokio::time::Instant::now().into_std(),
            ));
            assert_eq!(q.visible.peek()[0].count, 1);
            q.tick.call((
                None,
                tokio::time::Instant::now().into_std() + Duration::from_secs(8),
            ));
            assert_eq!(*q.visible.peek(), []);
            assert!(view.peek().error.as_ref().unwrap().contains("not saved"));
        });
}
