//! Stable logical slots survive missing controls and native inventory reordering.
use super::controllers::{ControlLayout, DeviceBinding};
use crate::types::InputId;

impl DeviceBinding {
    /// Reconcile a native inventory without ever reusing a logical slot.
    /// Positional APIs cannot prove identity after a layout change.
    pub fn reconcile(&mut self, current: &Self, stable_codes: bool) {
        let previous = self.unavailable.clone();
        self.unavailable.clear();
        let current_layout = ControlLayout::from(current);
        let observed = self
            .observed_layout
            .clone()
            .unwrap_or_else(|| ControlLayout::from(&*self));
        let axes_changed = observed.axes != current_layout.axes;
        let buttons_changed = observed.buttons != current_layout.buttons;
        let hats_changed = observed.hats != current_layout.hats;
        self.observed_layout = (!stable_codes).then_some(current_layout);
        for (i, axis) in self.axes.iter_mut().enumerate() {
            let input = InputId::Axis {
                index: u8::try_from(i).unwrap_or(u8::MAX),
            };
            if let Some(found) = current.axes.iter().find(|a| a.code == axis.code) {
                axis.minimum = found.minimum;
                axis.maximum = found.maximum;
            }
            if !current.axes.iter().any(|a| a.code == axis.code)
                || (!stable_codes && (axes_changed || previous.contains(&input)))
            {
                self.unavailable.push(input);
            }
        }
        for axis in &current.axes {
            if !self.axes.iter().any(|a| a.code == axis.code) && self.axes.len() < 255 {
                self.axes.push(axis.clone());
            }
        }
        reconcile_codes(
            &mut self.buttons,
            &current.buttons,
            stable_codes,
            buttons_changed,
            &previous,
            &mut self.unavailable,
            |index| InputId::Button { index },
            255,
        );
        reconcile_codes(
            &mut self.hats,
            &current.hats,
            stable_codes,
            hats_changed,
            &previous,
            &mut self.unavailable,
            |index| InputId::Hat { index },
            4,
        );
    }
}

impl From<&DeviceBinding> for ControlLayout {
    fn from(binding: &DeviceBinding) -> Self {
        Self {
            axes: binding.axes.iter().map(|axis| axis.code).collect(),
            buttons: binding.buttons.clone(),
            hats: binding.hats.clone(),
        }
    }
}
impl ControlLayout {
    pub(super) fn valid_for(&self, binding: &DeviceBinding) -> bool {
        let unique_subset = |observed: &[u16], frozen: &[u16]| {
            observed
                .iter()
                .enumerate()
                .all(|(index, code)| frozen.contains(code) && !observed[..index].contains(code))
        };
        unique_subset(
            &self.axes,
            &binding
                .axes
                .iter()
                .map(|axis| axis.code)
                .collect::<Vec<_>>(),
        ) && unique_subset(&self.buttons, &binding.buttons)
            && self
                .hats
                .iter()
                .enumerate()
                .all(|(index, hat)| binding.hats.contains(hat) && !self.hats[..index].contains(hat))
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "shared button/hat reconciliation with stable-slot context"
)]
fn reconcile_codes<T: Copy + PartialEq>(
    saved: &mut Vec<T>,
    current: &[T],
    stable: bool,
    changed: bool,
    previous: &[InputId],
    missing: &mut Vec<InputId>,
    input: impl Fn(u8) -> InputId,
    limit: usize,
) {
    for (i, code) in saved.iter().enumerate() {
        let id = input(u8::try_from(i).unwrap_or(u8::MAX));
        if !current.contains(code) || (!stable && (changed || previous.contains(&id))) {
            missing.push(id);
        }
    }
    for code in current {
        if !saved.contains(code) && saved.len() < limit {
            saved.push(*code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceId;
    fn table(buttons: Vec<u16>) -> DeviceBinding {
        DeviceBinding {
            observed_layout: None,
            device: DeviceId("test".into()),
            axes: vec![],
            buttons,
            hats: vec![],
            unavailable: vec![],
        }
    }
    #[test]
    fn missing_slots_are_not_reused_and_returning_controls_recover() {
        let mut saved = table(vec![10, 20]);
        saved.reconcile(&table(vec![5, 20]), true);
        assert_eq!(saved.buttons, [10, 20, 5]);
        assert!(!saved.resolves(&InputId::Button { index: 0 }));
        assert!(saved.resolves(&InputId::Button { index: 2 }));
        saved.reconcile(&table(vec![5, 10, 20]), true);
        assert_eq!(saved.buttons, [10, 20, 5]);
        assert!(saved.unavailable.is_empty());
    }
    #[test]
    fn legacy_layout_metadata_is_optional_and_unknown_observed_codes_are_rejected() {
        let original = table(vec![0, 1]);
        let serialized = toml::to_string(&original).unwrap();
        assert!(!serialized.contains("observed_layout"));
        let mut legacy: DeviceBinding = toml::from_str(&serialized).unwrap();
        assert_eq!(legacy, original);
        legacy.reconcile(&table(vec![0]), false);
        assert!(!legacy.resolves(&InputId::Button { index: 0 }));
        legacy.observed_layout.as_mut().unwrap().buttons.push(99);
        assert!(legacy.validate().is_err());
    }
    #[test]
    fn positional_layout_changes_stay_ambiguous_until_explicit_rebinding() {
        let mut saved = table(vec![0, 1]);
        saved.reconcile(&table(vec![0, 1, 2]), false);
        assert!(!saved.resolves(&InputId::Button { index: 0 }));
        saved.reconcile(&table(vec![0, 1, 2]), false);
        assert!(!saved.resolves(&InputId::Button { index: 0 }));
    }
}
