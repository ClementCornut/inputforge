//! SDL positional identities stay disabled until the user confirms a live control.
use crate::{
    error::{EngineError, Result},
    profile::controllers::{AxisBinding, DeviceBinding},
    types::{AxisPolarity, InputId},
};
pub(super) fn binding(info: &crate::types::DeviceInfo) -> DeviceBinding {
    DeviceBinding {
        observed_layout: None,
        device: info.id.clone(),
        axes: (0..info.axes)
            .map(|index| AxisBinding {
                code: u16::from(index),
                minimum: i32::from(i16::MIN),
                maximum: i32::from(i16::MAX),
                polarity: AxisPolarity::default(),
            })
            .collect(),
        buttons: (0..info.buttons).map(u16::from).collect(),
        hats: (0..info.hats).collect(),
        unavailable: Vec::new(),
    }
}
pub(super) fn index(binding: &DeviceBinding, input: &InputId) -> Option<InputId> {
    let mapped = match input {
        InputId::Axis { index } => InputId::Axis {
            index: u8::try_from(
                binding
                    .axes
                    .iter()
                    .position(|a| a.code == u16::from(*index))?,
            )
            .ok()?,
        },
        InputId::Button { index } => InputId::Button {
            index: u8::try_from(
                binding
                    .buttons
                    .iter()
                    .position(|b| *b == u16::from(*index))?,
            )
            .ok()?,
        },
        InputId::Hat { index } => InputId::Hat {
            index: u8::try_from(binding.hats.iter().position(|h| *h == *index)?).ok()?,
        },
    };
    Some(mapped)
}
pub(super) fn confirm(
    saved: &mut DeviceBinding,
    current: &DeviceBinding,
    input: &InputId,
) -> Result<DeviceBinding> {
    let present = match input {
        InputId::Axis { index } => saved
            .axes
            .get(usize::from(*index))
            .is_some_and(|a| current.axes.iter().any(|c| c.code == a.code)),
        InputId::Button { index } => saved
            .buttons
            .get(usize::from(*index))
            .is_some_and(|b| current.buttons.contains(b)),
        InputId::Hat { index } => saved
            .hats
            .get(usize::from(*index))
            .is_some_and(|h| current.hats.contains(h)),
    };
    if !present {
        return Err(EngineError::InvalidConfig {
            reason: "The selected native input is no longer present".into(),
        });
    }
    saved.unavailable.retain(|item| item != input);
    Ok(saved.clone())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, DeviceInfo};
    fn table(buttons: u8) -> DeviceBinding {
        layout(1, buttons, 1)
    }
    fn layout(axes: u8, buttons: u8, hats: u8) -> DeviceBinding {
        binding(&DeviceInfo {
            id: DeviceId("fixture".into()),
            name: "fixture".into(),
            axes,
            buttons,
            hats,
            instance_path: None,
            axis_polarities: vec![],
        })
    }
    #[test]
    fn ambiguous_controls_preview_but_only_explicit_confirmation_enables_routing() {
        let mut saved = table(2);
        let current = table(3);
        saved.reconcile(&current, false);
        let button = InputId::Button { index: 0 };
        assert_eq!(index(&saved, &button), Some(button.clone()));
        assert!(!saved.resolves(&button));
        let confirmed = confirm(&mut saved, &current, &button).unwrap();
        assert!(confirmed.resolves(&button));
        assert!(!confirmed.resolves(&InputId::Button { index: 1 }));
        assert!(confirmed.resolves(&InputId::Button { index: 2 }));
        saved.reconcile(&current, false);
        assert!(saved.resolves(&button));
        assert!(!saved.resolves(&InputId::Button { index: 1 }));
    }
    #[test]
    fn shrink_confirmation_survives_reload_but_new_layout_changes_require_confirmation() {
        let mut saved = layout(3, 3, 3);
        let current = layout(1, 1, 1);
        saved.reconcile(&current, false);
        let first = [
            InputId::Axis { index: 0 },
            InputId::Button { index: 0 },
            InputId::Hat { index: 0 },
        ];
        for input in &first {
            confirm(&mut saved, &current, input).unwrap();
        }
        let serialized = toml::to_string(&saved).unwrap();
        let mut restored: DeviceBinding = toml::from_str(&serialized).unwrap();
        restored.reconcile(&current, false);
        restored.validate().unwrap();
        for input in &first {
            assert!(restored.resolves(input));
        }
        for input in [
            InputId::Axis { index: 2 },
            InputId::Button { index: 2 },
            InputId::Hat { index: 2 },
        ] {
            assert!(!restored.resolves(&input));
            confirm(&mut restored, &current, &input).unwrap_err();
        }
        assert_eq!(
            restored.axes.iter().map(|a| a.code).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(restored.buttons, [0, 1, 2]);
        assert_eq!(restored.hats, [0, 1, 2]);
        for (kind, changed) in [layout(2, 1, 1), layout(1, 2, 1), layout(1, 1, 2)]
            .iter()
            .enumerate()
        {
            let mut changed_saved = restored.clone();
            changed_saved.reconcile(changed, false);
            for (input_kind, input) in first.iter().enumerate() {
                assert_eq!(changed_saved.resolves(input), input_kind != kind);
            }
        }
    }

    #[test]
    fn missing_control_cannot_be_confirmed() {
        let mut saved = table(3);
        let current = table(1);
        saved.reconcile(&current, false);
        let before = saved.clone();
        confirm(&mut saved, &current, &InputId::Button { index: 2 }).unwrap_err();
        assert_eq!(saved, before);
    }
}
