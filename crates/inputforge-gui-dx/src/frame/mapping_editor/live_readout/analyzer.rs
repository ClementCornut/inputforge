// Rust guideline compliant 2026-05-03

#![expect(
    dead_code,
    reason = "Task 4 defines the readout model before the analyzer walker uses it."
)]

use inputforge_core::types::{AxisPolarity, InputAddress, KeyCombo, MergeOp, OutputAddress};

/// Maximum action nesting analyzed for live readout.
///
/// This is high enough for editor built pipelines while bounding future
/// recursive walks through nested conditionals.
pub(super) const MAX_NESTED_ACTION_DEPTH: usize = 32;

/// Complete live readout analysis result.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct LiveReadoutModel {
    /// Inputs shown at the head of the pipeline.
    pub pipeline_inputs: Vec<InputAddress>,
    /// Conditions evaluated while shaping output rows.
    pub predicates: Vec<PredicateDescriptor>,
    /// Output rows derived from terminal mapping actions.
    pub outputs: Vec<OutputDescriptor>,
}

/// Output row with its action chain.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct OutputDescriptor {
    /// Target reached by the analyzed pipeline.
    pub destination: OutputDestination,
    /// Merge and conditional steps that affect this output.
    pub chain: Vec<ChainStep>,
    /// Whether the current action path can produce this output.
    pub is_active: bool,
    /// Polarity used when rendering the final output value.
    pub polarity: AxisPolarity,
}

/// Destination kind for output rows.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum OutputDestination {
    /// vJoy axis, button, or hat output.
    VJoy(OutputAddress),
    /// Keyboard key combination output.
    Keyboard(KeyCombo),
}

/// Transformation step shown in an output chain.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ChainStep {
    /// Merge applied to the primary pipeline value.
    Merge {
        /// Operation selected by the merge stage.
        operation: MergeOp,
        /// Secondary input consumed by this merge.
        secondary_input: InputAddress,
        /// Secondary value before display domain conversion.
        encoded_value: f64,
        /// Output polarity after applying this merge.
        polarity_at_step: AxisPolarity,
    },
    /// Conditional branch taken or skipped during analysis.
    Conditional {
        /// User visible condition summary.
        condition_label: String,
        /// Result of evaluating the condition.
        evaluated: bool,
        /// Branch selected by the condition outcome.
        branch: Branch,
    },
}

/// Branch selected by a conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Branch {
    /// The true branch of a condition.
    IfTrue,
    /// The false branch of a condition.
    IfFalse,
}

/// Predicate row shown beside conditional output state.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PredicateDescriptor {
    /// Predicate behavior used for dedup and display.
    pub kind: PredicateKind,
    /// Inputs read while evaluating this predicate.
    pub inputs: Vec<InputAddress>,
    /// Current evaluated state.
    pub state: bool,
    /// User visible predicate summary.
    pub label: String,
}

/// Predicate variants captured by the analyzer.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PredicateKind {
    /// Button must be currently pressed.
    ButtonPressed,
    /// Button must be currently released.
    ButtonReleased,
    /// Axis must fall inside the inclusive range.
    AxisInRange {
        /// Inclusive lower bound.
        min: f64,
        /// Inclusive upper bound.
        max: f64,
    },
    /// Hat must match one of these directions.
    HatDirection {
        /// Accepted hat directions.
        directions: Vec<inputforge_core::types::HatDirection>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId, OutputId, VJoyAxis};

    #[test]
    fn model_default_is_empty() {
        let model = LiveReadoutModel::default();

        assert!(model.pipeline_inputs.is_empty());
        assert!(model.predicates.is_empty());
        assert!(model.outputs.is_empty());
    }

    #[test]
    fn output_descriptor_carries_destination_chain_polarity() {
        let secondary_input = InputAddress::Bound {
            device: DeviceId("stick".into()),
            input: InputId::Axis { index: 1 },
        };
        let output = OutputDescriptor {
            destination: OutputDestination::VJoy(OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
            chain: vec![ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: secondary_input.clone(),
                encoded_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }],
            is_active: true,
            polarity: AxisPolarity::Bipolar,
        };

        assert_eq!(output.chain.len(), 1);
        assert_eq!(output.polarity, AxisPolarity::Bipolar);
        assert!(output.is_active);
        assert_eq!(
            output.chain[0],
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input,
                encoded_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }
        );
    }
}
