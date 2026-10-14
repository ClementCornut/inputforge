//! Prepared drag identities change when a reusable stage represents new content.
use super::tests::{build_state, render_with};
use inputforge_core::{
    action::Action,
    state::AppState,
    types::{DeviceId, InputAddress, InputId},
};

fn context(state: AppState, address: InputAddress) -> String {
    let html = scraper::Html::parse_fragment(&render_with(state, address));
    let selector = scraper::Selector::parse("[data-sortable-group]").unwrap();
    let group = html.select(&selector).next().unwrap();
    let context = group.value().attr("data-sortable-context");
    assert!(
        context.is_some(),
        "Pipeline groups need the prepared source identity"
    );
    let context = context.unwrap();
    let selector = scraper::Selector::parse(".if-sortable-handle").unwrap();
    assert!(
        html.select(&selector)
            .all(|handle| handle.value().attr("data-sortable-context") == Some(context))
    );
    context.to_owned()
}

#[test]
fn pipeline_context_tracks_profile_mapping_and_action_tree_without_remounting_for_geometry() {
    let (state, address) = build_state(vec![Action::Invert, Action::Invert]);
    let profile = state.active_profile.unwrap();
    let before = context(AppState::with_profile(profile.clone()), address.clone());
    let mut edited = profile.clone();
    edited.set_mapping(&address, "Default", None, vec![Action::Invert]);
    assert_ne!(
        context(AppState::with_profile(edited), address.clone()),
        before
    );
    let other = InputAddress::Bound {
        device: DeviceId("dev-1".into()),
        input: InputId::Axis { index: 1 },
    };
    let mut mappings = profile;
    mappings.set_mapping(
        &other,
        "Default",
        None,
        vec![Action::Invert, Action::Invert],
    );
    assert_ne!(context(AppState::with_profile(mappings), other), before);
    let (state, address) = build_state(vec![Action::Invert, Action::Invert]);
    assert_ne!(context(state, address), before);
}
