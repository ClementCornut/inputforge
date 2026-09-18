use inputforge_core::device::evdev::{NativeControl, NativeState, SnapshotKind, StreamUpdate};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

// Bound historical frame storage; aggregate counts continue after the sample fills.
const SAMPLE_LIMIT: usize = 256;

#[derive(Debug, Default)]
pub(super) struct Report {
    devices: BTreeMap<String, DeviceReport>,
    frames: Vec<StreamUpdate>,
    omitted: u64,
    ready: bool,
}

#[derive(Debug, Default)]
struct DeviceReport {
    initial: Option<NativeState>,
    final_state: Option<NativeState>,
    resets: u64,
    controls: BTreeMap<NativeControl, Counts>,
}

#[derive(Debug)]
struct Counts {
    events: u64,
    min: i32,
    max: i32,
}

impl Report {
    pub(super) fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    pub(super) fn observe(&mut self, update: StreamUpdate) {
        match &update {
            StreamUpdate::Frame {
                device, changes, ..
            } => {
                let report = self.devices.entry(device.0.clone()).or_default();
                for change in changes {
                    report.record(change.control, change.value, true);
                    if let Some(state) = &mut report.final_state {
                        match change.control {
                            NativeControl::Key(code) => {
                                state.keys.insert(code, change.value == 1);
                            }
                            NativeControl::Abs(code) => {
                                state.axes.insert(code, change.value);
                            }
                        }
                    }
                }
                if self.frames.len() < SAMPLE_LIMIT {
                    self.frames.push(update);
                } else {
                    self.omitted += 1;
                }
            }
            StreamUpdate::Snapshot {
                device,
                state,
                kind,
                ..
            } => {
                let report = self.devices.entry(device.0.clone()).or_default();
                if *kind == SnapshotKind::Initial {
                    report.initial = Some(state.clone());
                }
                for (&code, &value) in &state.keys {
                    report.record(NativeControl::Key(code), i32::from(value), false);
                }
                for (&code, &value) in &state.axes {
                    report.record(NativeControl::Abs(code), value, false);
                }
                report.final_state = Some(state.clone());
            }
            StreamUpdate::Reset { device } => {
                let report = self.devices.entry(device.0.clone()).or_default();
                report.resets += 1;
                report.final_state = None;
            }
        }
    }

    pub(super) fn invalidate(&mut self) {
        self.ready = false;
        for report in self.devices.values_mut() {
            report.final_state = None;
        }
    }

    pub(super) fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(
            writer,
            "stream history (ownership released; final means last valid sampled state):"
        )?;
        writeln!(writer, "all_ready_at_end={}", self.ready)?;
        writeln!(
            writer,
            "sampled_frames={} omitted_frames={}",
            self.frames.len(),
            self.omitted
        )?;
        for (id, report) in &self.devices {
            writeln!(writer, "device={id:?} resets={}", report.resets)?;
            writeln!(writer, "  initial={:?}", report.initial)?;
            writeln!(writer, "  final={:?}", report.final_state)?;
            for (control, counts) in &report.controls {
                writeln!(
                    writer,
                    "  {control:?} events={} min={} max={}",
                    counts.events, counts.min, counts.max
                )?;
            }
        }
        for frame in &self.frames {
            writeln!(writer, "  historical_sample={frame:?}")?;
        }
        writer.flush()
    }
}

impl DeviceReport {
    fn record(&mut self, control: NativeControl, value: i32, event: bool) {
        let counts = self.controls.entry(control).or_insert(Counts {
            events: 0,
            min: value,
            max: value,
        });
        counts.events += u64::from(event);
        counts.min = counts.min.min(value);
        counts.max = counts.max.max(value);
    }
}
