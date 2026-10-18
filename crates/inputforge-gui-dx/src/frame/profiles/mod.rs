use crate::{
    context::AppContext,
    profiles::{Bindings, Context, Mode, View},
};
use dioxus::prelude::*;
use library::ProfileLibrary;
use new_profile_submode::NewProfileSubMode;
use open_choice_submode::OpenChoiceSubMode;
use snapshot_drawer::SnapshotDrawer;

pub(crate) mod delete_dialog;
pub(crate) mod library;
pub(crate) mod new_profile_submode;
pub(crate) mod open_choice_submode;
mod row;
mod snapshot_confirm;
pub(crate) mod snapshot_drawer;
mod snapshot_row;
mod source_options;

/// Unit fixtures use the real app host, while desktop hosts supply bindings.
pub(crate) fn use_bindings() -> Bindings {
    #[cfg(test)]
    if try_consume_context::<Bindings>().is_none() {
        let controller = use_hook(|| {
            inputforge_app::profiles::Controller::new(
                crate::test_support::engine().state,
                crate::test_support::engine().commands,
            )
        });
        return controller.bindings();
    }
    use_context::<Bindings>()
}
#[component]
pub(crate) fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let bindings = use_bindings();
    let view = use_signal(View::default);
    let owner = use_hook(|| bindings.mount.call(view));
    use_drop(move || bindings.unmount.call(owner));
    let c = Context {
        view,
        owner,
        workflow: 0,
        bindings,
    };
    use_context_provider(|| c);
    let prepared = view.read().clone();
    let library_error = prepared.error.clone().filter(|_| {
        prepared.mode == Mode::Library
            && prepared.delete.is_none()
            && prepared.snapshots.restore.is_none()
    });
    let meta = ctx.meta.read();
    let has_profile = meta.profile_name.is_some();
    let snapshot_count = meta.snapshot_rows.len();
    rsx! {
        section {
            class: "profiles-panel",
            "data-testid": "profile-library",
            div {
                class: "profiles-panel__body",
                match prepared.mode {
                    Mode::New => rsx! {
                        NewProfileSubMode {
                        }
                    },
                    Mode::Open { path } => rsx! {
                        OpenChoiceSubMode {
                            path,
                        }
                    },
                    Mode::Library => rsx! { ProfileLibrary {} },
                }
            }
            if let Some(error) = library_error {
                crate::components::InlineFeedback { tone: crate::components::FeedbackTone::Error, alert: true, "{error}" }
            }
            if has_profile {
                SnapshotDrawer {}
            } else {
                footer {
                    class: "profiles-panel__snapshot-toggle",
                    "Load a profile to view snapshots ({snapshot_count})"
                }
            }
            if prepared.snapshots.restore.is_some() {
                snapshot_confirm::SnapshotConfirm {}
            }
            if prepared.delete.is_some() {
                delete_dialog::ProfileDeleteDialog {}
            }
        }
    }
}
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod snapshot_events;
