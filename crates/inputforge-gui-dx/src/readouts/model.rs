//! Completed action-tree analysis supplied by the application.
use inputforge_core::{
    action::{ActionBranch, MouseTarget, OutputBehavior},
    state::OutputActivityValue,
    types::{AxisPolarity, HatDirection, InputAddress, KeyCombo, MergeOp, OutputAddress},
};
/// Complete live readout analysis result.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiveReadoutModel {
    /// Inputs shown at the head of the pipeline.
    pub pipeline_inputs: Vec<InputAddress>,
    /// Conditions evaluated while shaping output rows.
    pub predicates: Vec<PredicateDescriptor>,
    /// Output rows derived from terminal mapping actions.
    pub outputs: Vec<OutputDescriptor>,
}

/// Output row with its action chain.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputDescriptor {
    /// Exact runtime owner for activity attribution.
    pub owner: inputforge_core::pipeline::OutputOwner,
    /// Target reached by the analyzed pipeline.
    pub destination: OutputDestination,
    /// Merge and conditional steps that affect this output.
    pub chain: Vec<ChainStep>,
    /// Whether the current action path can produce this output.
    pub is_active: bool,
    /// Polarity used when rendering the final output value.
    pub polarity: AxisPolarity,
    /// Recent runtime output activity for this exact action path.
    pub activity: Option<OutputActivityValue>,
}

/// Destination kind for output rows.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputDestination {
    /// vJoy axis, button, or hat output.
    VJoy(OutputAddress),
    /// Keyboard key combination output.
    Keyboard {
        /// Key combination emitted by the pipeline.
        key: KeyCombo,
        /// Whether the keyboard output holds or pulses.
        behavior: OutputBehavior,
        /// Whether the current pipeline value would press the key.
        pressed: bool,
    },
    /// Mouse button or wheel output.
    Mouse {
        /// Mouse target emitted by the pipeline.
        target: MouseTarget,
        /// Whether the mouse output holds or pulses.
        behavior: OutputBehavior,
        /// Whether the current pipeline value would activate the target.
        active: bool,
    },
}

/// Transformation step shown in an output chain.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainStep {
    /// Merge applied to the primary pipeline value.
    Merge {
        /// Operation selected by the merge stage.
        operation: MergeOp,
        /// Secondary input consumed by this merge.
        secondary_input: InputAddress,
        /// Running merge result in the displayed natural domain.
        display_value: f64,
        /// Output polarity after applying this merge.
        polarity_at_step: AxisPolarity,
    },
    /// Conditional branch taken or skipped during analysis.
    Conditional {
        /// User visible condition summary.
        condition_label: String,
        /// Whether this exact branch is currently active.
        active: bool,
        /// Branch selected by the condition outcome.
        branch: Branch,
    },
}

/// Branch selected by a conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// The true branch of a condition.
    IfTrue,
    /// The false branch of a condition.
    IfFalse,
    /// A gesture outcome branch; runtime gesture telemetry is not sampled here.
    Gesture(ActionBranch),
}

/// Predicate row shown beside conditional output state.
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateDescriptor {
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
pub enum PredicateKind {
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
        directions: Vec<HatDirection>,
    },
}
