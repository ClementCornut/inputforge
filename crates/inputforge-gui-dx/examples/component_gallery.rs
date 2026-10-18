//! Interactive design-system gallery, with app-owned preview bindings and no hardware.
use dioxus::prelude::*;
use inputforge_gui_dx::{ToastViewport, components::Stack, theme::ThemeProvider};
#[path = "component_gallery/actions.rs"]
mod actions;
#[path = "component_gallery/choices.rs"]
mod choices;
#[path = "component_gallery/dialogs.rs"]
mod dialogs;
#[path = "component_gallery/feedback.rs"]
mod feedback;
#[path = "component_gallery/navigation.rs"]
mod navigation;
#[path = "component_gallery/notifications.rs"]
mod notifications;
#[path = "component_gallery/selects.rs"]
mod selects;
#[path = "component_gallery/text_numbers.rs"]
mod text_numbers;

#[path = "component_gallery/editing.rs"]
mod editing;
#[path = "component_gallery/surfaces.rs"]
mod surfaces;

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();
    LaunchBuilder::desktop().launch(gallery_root);
}
fn gallery_root() -> Element {
    inputforge_app::ui::use_provider();
    let _ = inputforge_app::notifications::use_preview();
    rsx! {
        ThemeProvider {
            ToastViewport {}
            main {
                Stack { gap: "--space-8", padding: "--space-6",
                    h1 { "InputForge Component Gallery" }
                    notifications::NotificationsGallery {}
                    dialogs::DialogsGallery {}
                    actions::ActionsGallery {}
                    text_numbers::TextNumbersGallery {}
                    selects::SelectsGallery {}
                    choices::ChoicesGallery {}
                    feedback::FeedbackGallery {}
                    navigation::NavigationGallery {}
                    editing::EditingGallery {}
                    surfaces::SurfacesGallery {}
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "component_gallery/accessibility_tests.rs"]
mod accessibility_tests;
