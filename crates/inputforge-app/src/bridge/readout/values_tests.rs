//! Moved sampled value and polarity regressions.
use super::{outputs::*, polarity::*};
use inputforge_core::types::{AxisPolarity, HatDirection, MergeOp, OutputAddress, OutputId};
use inputforge_gui_dx::{
    readouts::ReadoutDisplay,
    snapshots::{ConfigSnapshot, LiveSnapshot},
};
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::VirtualDeviceConfig;
    use inputforge_gui_dx::snapshots::VjoyOutputValues;

    #[test]
    fn read_output_typed_display_reads_one_based_hat_output() {
        let cfg = ConfigSnapshot {
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 0,
                hat_count: 1,
            }],
            ..ConfigSnapshot::default()
        };
        let live = LiveSnapshot {
            capture_epoch: inputforge_gui_dx::snapshots::CaptureEpoch::default(),
            device_inputs: vec![],
            output_values: vec![VjoyOutputValues {
                axes: vec![],
                buttons: vec![],
                hats: vec![HatDirection::SE],
            }],
            output_activity: vec![],
            ..LiveSnapshot::default()
        };
        let out = OutputAddress {
            device: 1,
            output: OutputId::Hat { id: 1 },
        };

        assert_eq!(
            read_output_typed_display(&out, &live, &cfg, AxisPolarity::Bipolar),
            ReadoutDisplay::Hat {
                direction: HatDirection::SE,
            }
        );
    }

    #[test]
    fn merge_output_polarity_bidirectional_always_bipolar() {
        for primary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
            for secondary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                assert_eq!(
                    merge_output_polarity(MergeOp::Bidirectional, primary, secondary),
                    AxisPolarity::Bipolar
                );
            }
        }
    }

    #[test]
    fn merge_output_polarity_average_uu_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_bb_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_uu_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_bb_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_and_maximum_are_commutative() {
        for op in [MergeOp::Average, MergeOp::Maximum] {
            for a in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                for b in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                    assert_eq!(
                        merge_output_polarity(op, a, b),
                        merge_output_polarity(op, b, a),
                        "{op:?}({a:?}, {b:?}) should equal {op:?}({b:?}, {a:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn infer_output_polarity_no_merges_inherits_primary() {
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &[]),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn infer_output_polarity_chained_merges_compose_left_to_right() {
        // Unipolar primary plus Bidirectional with Unipolar gives Bipolar.
        // Then Average with Unipolar keeps Bipolar because mixed inputs promote.
        let path = [
            (MergeOp::Bidirectional, AxisPolarity::Unipolar),
            (MergeOp::Average, AxisPolarity::Unipolar),
        ];
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &path),
            AxisPolarity::Bipolar
        );
    }
}
