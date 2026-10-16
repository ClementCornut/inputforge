//! Stable DOM IDs for plot geometry.
use crate::mapping_edits::{StageId, StageIdSegment};
use inputforge_core::action::ActionBranch;
use std::fmt::Write as _;
pub(crate) fn stage_id_dom_id(prefix: &str, stage_id: &StageId) -> String {
    let mut s = String::from(prefix);
    for seg in &stage_id.0 {
        match seg {
            StageIdSegment::Index(n) => {
                let _ = write!(s, "-i{n}");
            }
            StageIdSegment::Branch(ActionBranch::ConditionalTrue) => s.push_str("-t"),
            StageIdSegment::Branch(ActionBranch::ConditionalFalse) => s.push_str("-f"),
            StageIdSegment::Branch(ActionBranch::TapSingle) => s.push_str("-ts"),
            StageIdSegment::Branch(ActionBranch::TapDouble) => s.push_str("-td"),
            StageIdSegment::Branch(ActionBranch::PressShort) => s.push_str("-ps"),
            StageIdSegment::Branch(ActionBranch::PressLong) => s.push_str("-pl"),
        }
    }
    s
}
