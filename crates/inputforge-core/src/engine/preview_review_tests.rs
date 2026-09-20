//! Review evidence only: no changes to publication, rendering or output timing.
use super::*;
use crate::pipeline::InputCache;

struct SnapshotDuringOutput {
    state: Arc<RwLock<AppState>>,
    observed: Arc<Mutex<Vec<(bool, bool)>>>,
}
impl OutputSink for SnapshotDuringOutput {
    fn start(&mut self, _: &[VirtualDeviceConfig]) -> Result<()> {
        Ok(())
    }
    fn neutralize(&mut self) -> Result<()> {
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn set_axis(&mut self, _: u8, _: VJoyAxis, _: f64) -> Result<()> {
        Ok(())
    }
    fn set_hat(&mut self, _: u8, _: u8, _: HatDirection) -> Result<()> {
        Ok(())
    }
    fn set_button(&mut self, device: u8, button: u8, _: bool) -> Result<()> {
        // The bridge uses the same nonblocking read. This represents a GUI
        // poll occurring after input publication but before output publication.
        let state = self
            .state
            .try_read()
            .expect("bridge can read state during output dispatch");
        self.observed.lock().push((
            state.input_cache.get_button(&super::button(false).source),
            state.output_cache.get_button(device, button),
        ));
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn review_bridge_can_observe_new_input_before_its_output_is_published() {
    let (mut e, tx, script, _temp) = harness();
    let observed = Arc::new(Mutex::new(Vec::new()));
    e.output = Box::new(SnapshotDuringOutput {
        state: Arc::clone(&e.state),
        observed: Arc::clone(&observed),
    });
    tx.send(EngineCommand::Activate).unwrap();
    script.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    for pressed in [true, false] {
        script
            .lock()
            .polls
            .push_back(vec![InputUpdate::Frame(vec![button(pressed)])]);
        e.tick().unwrap();
        let state = e.state.read();
        assert_eq!(state.input_cache.get_button(&button(false).source), pressed);
        assert_eq!(state.output_cache.get_button(1, 1), pressed);
    }
    assert_eq!(*observed.lock(), [(true, false), (false, true)]);
}
