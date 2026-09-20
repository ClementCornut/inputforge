use super::super::{config, state::State};
use crate::types::{HatDirection, VJoyAxis};

fn records(state: &State, force: bool) -> Vec<Vec<(u16, u16, i32)>> {
    state
        .packets(force)
        .iter()
        .map(|packet| {
            packet
                .iter()
                .map(|event| (event.event_type().0, event.code(), event.value()))
                .collect()
        })
        .collect()
}

#[test]
fn axis_quantization_clamps_and_neutralizes_nonfinite_values() {
    let cases = [
        (-2.0, -32767),
        (-1.0, -32767),
        (-0.5, -16384),
        (-0.0, 0),
        (0.5, 16384),
        (1.0, 32767),
        (2.0, 32767),
        (f64::NAN, 0),
        (f64::INFINITY, 0),
        (f64::NEG_INFINITY, 0),
    ];
    for (value, expected) in cases {
        let mut state = State::new(&config::default_config(1));
        state.axis(VJoyAxis::Slider1, value).unwrap();
        let frames = records(&state, true);
        assert!(
            frames
                .iter()
                .flatten()
                .any(|&record| record == (3, 7, expected))
        );
    }
    for (value, expected) in [
        (0.49 / 32767.0, 0),
        (0.5 / 32767.0, 1),
        (-0.5 / 32767.0, -1),
    ] {
        let mut state = State::new(&config::default_config(1));
        state.axis(VJoyAxis::X, value).unwrap();
        assert!(
            records(&state, true)
                .iter()
                .flatten()
                .any(|&r| r == (3, 0, expected))
        );
    }
}

#[test]
fn sparse_buttons_preserve_logical_addresses_without_keyboard_codes() {
    let mut cfg = config::default_config(1);
    cfg.button_count = 53;
    let mut state = State::new(&cfg);
    for id in [12, 13, 14, 53] {
        state.button(id, true).unwrap();
    }
    assert_eq!(
        records(&state, false),
        vec![vec![
            (1, 0x12b, 1),
            (1, 0x12f, 1),
            (1, 0x2c0, 1),
            (1, 0x2e7, 1),
            (0, 0, 0)
        ]]
    );
    for id in [0, 54, 255] {
        assert!(state.button(id, true).is_err());
    }
    state.commit();
    assert!(records(&state, false).is_empty());
    state.button(12, false).unwrap();
    state.button(12, true).unwrap();
    assert!(records(&state, false).is_empty());
}

#[test]
fn every_hat_direction_and_pair_is_encoded_together() {
    use HatDirection::{Center, E, N, NE, NW, S, SE, SW, W};
    for (direction, x, y) in [
        (Center, 0, 0),
        (N, 0, -1),
        (NE, 1, -1),
        (E, 1, 0),
        (SE, 1, 1),
        (S, 0, 1),
        (SW, -1, 1),
        (W, -1, 0),
        (NW, -1, -1),
    ] {
        let mut state = State::new(&config::default_config(1));
        state.hat(4, direction).unwrap();
        let frames = records(&state, true);
        let packet = frames.iter().find(|p| p.contains(&(3, 22, x))).unwrap();
        assert!(packet.contains(&(3, 23, y)));
    }
    let mut state = State::new(&config::default_config(1));
    state.hat(1, NE).unwrap();
    state.commit();
    state.hat(1, E).unwrap();
    assert_eq!(
        records(&state, false),
        vec![vec![(3, 16, 1), (3, 17, 0), (0, 0, 0)]]
    );
    assert!(state.hat(0, N).is_err());
    assert!(state.hat(5, N).is_err());
}

#[test]
fn full_state_packets_have_bounded_keys_and_single_report_boundaries() {
    let mut cfg = config::default_config(1);
    cfg.button_count = 53;
    let state = State::new(&cfg);
    let packets = records(&state, true);
    assert_eq!(packets.len(), 8);
    assert_eq!(packets[0].iter().filter(|r| r.0 == 3).count(), 16);
    assert_eq!(packets.iter().flatten().filter(|r| r.0 == 1).count(), 53);
    for packet in packets {
        assert!(packet.iter().filter(|r| r.0 == 1).count() <= 7);
        assert_eq!(packet.last(), Some(&(0, 0, 0)));
        assert_eq!(packet.iter().filter(|r| r.0 == 0).count(), 1);
    }
}

#[test]
fn undeclared_axes_are_rejected_and_reset_replaces_pending_state() {
    let mut cfg = config::default_config(1);
    cfg.axes = vec![VJoyAxis::Z];
    let mut state = State::new(&cfg);
    assert!(state.axis(VJoyAxis::X, 1.0).is_err());
    state.axis(VJoyAxis::Z, 1.0).unwrap();
    state.button(1, true).unwrap();
    state.neutral();
    assert!(records(&state, false).is_empty());
}
