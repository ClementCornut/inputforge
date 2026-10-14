mod add;
mod dispatch;
mod queued;
mod reorder;
use super::support::*;
use inputforge_core::{
    profile::Profile,
    types::{DeviceId, InputAddress, InputId},
};
use inputforge_gui_dx::mapping_rail::{Lookup, Scope, Target};
use parking_lot::RwLock;
use std::sync::Arc;

fn address(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("rail".into()),
        input: InputId::Button { index },
    }
}
fn scope(state: &Arc<RwLock<inputforge_core::state::AppState>>) -> Scope {
    Scope {
        profile: state
            .read()
            .active_profile
            .as_ref()
            .map(Profile::id)
            .cloned(),
        mode: "Default".into(),
    }
}
fn target(state: &Arc<RwLock<inputforge_core::state::AppState>>) -> Target {
    Target {
        scope: scope(state),
        before: mapping(vec![inputforge_core::action::Action::Invert]),
    }
}
