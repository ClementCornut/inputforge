//! Thread-affine passive controller monitoring with explicit exclusive grabs.
use super::{Capture, NativeState, translate};
use crate::{
    device::{HotplugEvent, InputSource, InputUpdate},
    error::{EngineError, Result},
    profile::controllers::{DeviceBinding, invalid},
    types::{DeviceId, DeviceInfo, InputAddress, InputId, InputValue},
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct EvdevInput {
    pub(super) capture: Option<Capture>,
    pub(super) tables: Vec<DeviceBinding>,
    states: HashMap<DeviceId, NativeState>,
    pending_updates: Vec<InputUpdate>,
    pub(super) inventory: Vec<DeviceInfo>,
    pub(super) hotplug: Vec<HotplugEvent>,
}

impl EvdevInput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(super) fn from_capture(capture: Capture) -> Self {
        let mut source = Self::default();
        source
            .replace_capture(capture)
            .expect("fixture passive monitoring");
        source
    }

    pub(super) fn replace_capture(&mut self, mut capture: Capture) -> Result<()> {
        capture
            .start_monitoring()
            .map_err(|error| native_error(&error))?;
        self.capture = Some(capture);
        self.states.clear();
        self.pending_updates.clear();
        self.publish_inventory();
        Ok(())
    }

    fn poll_inner(&mut self, out: &mut Vec<InputUpdate>) -> Result<()> {
        if self.capture.is_none() {
            self.refresh()?;
        }
        let Some(capture) = self.capture.as_mut() else {
            return Ok(());
        };
        let mut native = Vec::new();
        let status = capture.poll_monitor(&mut native);
        self.publish_inventory();
        out.append(&mut self.pending_updates);
        for update in native {
            let translated = translate::translate(&self.tables, &mut self.states, update)?;
            if let InputUpdate::Snapshot {
                values,
                recovered: false,
                ..
            } = &translated
            {
                for event in values {
                    if let (
                        InputAddress::Bound {
                            device,
                            input: InputId::Axis { index },
                        },
                        InputValue::Axis { value, .. },
                    ) = (&event.source, &event.value)
                    {
                        out.push(InputUpdate::AxisSample {
                            device: device.clone(),
                            index: *index,
                            value: value.value(),
                        });
                    }
                }
            }
            out.push(translated);
        }
        status.map(|_| ()).map_err(|error| native_error(&error))
    }
}

impl InputSource for EvdevInput {
    fn supports_exclusive(&self) -> bool {
        true
    }
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.inventory.clone()
    }
    fn is_device_connected(&self, id: &DeviceId) -> bool {
        self.inventory.iter().any(|d| &d.id == id)
    }
    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        std::mem::take(&mut self.hotplug)
    }
    fn configure(&mut self, bindings: &[DeviceBinding]) -> Result<()> {
        for table in bindings {
            table.validate()?;
        }
        self.pending_updates.clear();
        self.tables = bindings.to_vec();
        self.publish_inventory();
        Ok(())
    }
    fn refresh(&mut self) -> Result<()> {
        if self
            .capture
            .as_ref()
            .is_some_and(|c| !c.captured().is_empty())
        {
            return Err(invalid("Stop before refreshing capture"));
        }
        let capture = Capture::new().map_err(|error| native_error(&error))?;
        self.replace_capture(capture)
    }
    fn binding_table(&self, id: &DeviceId) -> Result<Option<DeviceBinding>> {
        Ok(self.tables.iter().find(|t| &t.device == id).cloned())
    }
    fn acquire(&mut self, ids: &[DeviceId]) -> Result<()> {
        if self.capture.is_none() {
            self.refresh()?;
        }
        let capture = self
            .capture
            .as_mut()
            .ok_or_else(|| invalid("Refresh controller discovery before retrying"))?;
        let result = capture.acquire(ids).map_err(|error| native_error(&error));
        self.publish_inventory();
        result
    }
    fn release(&mut self) -> Result<()> {
        self.capture.as_mut().map_or(Ok(()), |c| {
            c.release().map_err(|error| native_error(&error))
        })
    }
    fn request_axis_sample(&mut self, device: &DeviceId, index: u8) -> Result<()> {
        let table = self
            .tables
            .iter()
            .find(|t| &t.device == device)
            .ok_or_else(|| invalid("controller is unavailable"))?;
        if !table.resolves(&InputId::Axis { index }) {
            return Err(invalid("axis is unavailable"));
        }
        let axis = &table.axes[usize::from(index)];
        let raw = self
            .capture
            .as_ref()
            .and_then(|c| c.stream_state(device))
            .and_then(|s| s.axes.get(&axis.code))
            .ok_or_else(|| invalid("axis has no synchronized native sample"))?;
        let value = 2.0 * (f64::from(*raw) - f64::from(axis.minimum))
            / (f64::from(axis.maximum) - f64::from(axis.minimum))
            - 1.0;
        self.pending_updates.push(InputUpdate::AxisSample {
            device: device.clone(),
            index,
            value,
        });
        Ok(())
    }
    fn poll(&mut self, out: &mut Vec<InputUpdate>) -> Result<()> {
        let start = out.len();
        match self.poll_inner(out) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.pending_updates.extend(
                    out.drain(start..)
                        .filter(|update| !matches!(update, InputUpdate::Frame(_))),
                );
                let cleanup = self.release();
                self.publish_inventory();
                if let Err(cleanup) = cleanup {
                    return Err(invalid(format!("{error}; cleanup: {cleanup}")));
                }
                Err(error)
            }
        }
    }
}
fn native_error(error: &super::CaptureError) -> EngineError {
    EngineError::InputFailed {
        reason: format!("{error}; check connection/access or competing capture, then Retry"),
    }
}
