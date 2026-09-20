use super::{bindings, source::EvdevInput};
use crate::{device::HotplugEvent, types::DeviceInfo};

impl EvdevInput {
    pub(super) fn publish_inventory(&mut self) {
        let Some(capture) = &self.capture else {
            return;
        };
        let mut next = Vec::new();
        for device in capture.devices() {
            let Some(id) = &device.identity.id else {
                continue;
            };
            if device.classification.kind != super::Class::Controller {
                continue;
            }
            let mut capabilities_changed = false;
            let discovered = bindings::from_device(device);
            if let Some(saved) = self.tables.iter_mut().find(|b| &b.device == id) {
                let previous = saved.clone();
                let current =
                    discovered.unwrap_or_else(|_| crate::profile::controllers::DeviceBinding {
                        observed_layout: None,
                        device: id.clone(),
                        axes: vec![],
                        buttons: vec![],
                        hats: vec![],
                        unavailable: vec![],
                    });
                saved.reconcile(&current, true);
                capabilities_changed = *saved != previous;
            } else if let Ok(discovered) = discovered {
                self.tables.push(discovered);
            }
            let table = self.tables.iter().find(|b| &b.device == id);
            let info = DeviceInfo {
                id: id.clone(),
                name: device.metadata.name.clone(),
                axes: table
                    .as_ref()
                    .map_or(0, |t| u8::try_from(t.axes.len()).unwrap_or(0)),
                buttons: table
                    .as_ref()
                    .map_or(0, |t| u8::try_from(t.buttons.len()).unwrap_or(0)),
                hats: table
                    .as_ref()
                    .map_or(0, |t| u8::try_from(t.hats.len()).unwrap_or(0)),
                instance_path: Some(device.metadata.node.display().to_string()),
                axis_polarities: table
                    .as_ref()
                    .map_or_else(Vec::new, |t| t.axes.iter().map(|a| a.polarity).collect()),
            };
            if capabilities_changed || !self.inventory.contains(&info) {
                self.hotplug.push(HotplugEvent::Connected {
                    info: info.clone(),
                    diagnostics: device.metadata.diagnostics.clone(),
                });
            }
            next.push(info);
        }
        for old in &self.inventory {
            if !next.iter().any(|d| d.id == old.id) {
                self.hotplug
                    .push(HotplugEvent::Disconnected(old.id.clone()));
            }
        }
        self.inventory = next;
    }
}
