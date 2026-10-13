//! A deliberate rebind can confirm one control whose native layout changed.
use inputforge_core::{engine::EngineCommand, state::SessionState, types::InputAddress};
use std::sync::mpsc::{SendError, Sender};

/// Queue confirmation before changing a mapping, including same-address rebinds.
/// The engine validates physical presence before clearing the ambiguity.
pub(crate) fn confirm_binding(
    commands: &Sender<EngineCommand>,
    session: &SessionState,
    address: &InputAddress,
) -> Result<(), SendError<EngineCommand>> {
    if let InputAddress::Bound { device, input } = address
        && session
            .bindings
            .iter()
            .any(|table| &table.device == device && table.unavailable.contains(input))
    {
        commands.send(EngineCommand::ConfirmInputBinding {
            device: device.clone(),
            input: input.clone(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::{
        profile::controllers::DeviceBinding,
        types::{DeviceId, InputId},
    };
    use std::sync::mpsc;

    #[test]
    fn only_the_captured_ambiguous_control_is_confirmed() {
        let device = DeviceId("stick".to_owned());
        let input = InputId::Button { index: 0 };
        let other = InputId::Button { index: 1 };
        let session = SessionState {
            bindings: vec![DeviceBinding {
                observed_layout: None,
                device: device.clone(),
                axes: vec![],
                buttons: vec![0, 1],
                hats: vec![],
                unavailable: vec![input.clone(), other],
            }],
            ..SessionState::default()
        };
        let (commands, received) = mpsc::channel();
        confirm_binding(
            &commands,
            &session,
            &InputAddress::Bound {
                device: device.clone(),
                input: input.clone(),
            },
        )
        .unwrap();
        assert!(
            matches!(received.try_recv().unwrap(), EngineCommand::ConfirmInputBinding { device: id, input: confirmed } if id == device && confirmed == input)
        );
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        confirm_binding(
            &commands,
            &session,
            &InputAddress::Bound {
                device,
                input: InputId::Axis { index: 0 },
            },
        )
        .unwrap();
        confirm_binding(&commands, &session, &InputAddress::Unbound).unwrap();
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    }
}
