use super::{MenuItem, MenuItems, MenuRoot, MenuTrigger};
use dioxus::prelude::*;

#[test]
fn create_trigger_supports_compact_and_emphasized_shared_actions() {
    fn fixture() -> Element {
        inputforge_app::ui::use_provider();
        rsx! {
            MenuRoot {
                MenuTrigger { presentation: super::MenuTriggerPresentation::Create, compact: true, "Add stage" }
            }
            MenuRoot {
                MenuTrigger { presentation: super::MenuTriggerPresentation::Create, emphasized: true, "Choose first stage" }
            }
        }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let doc = scraper::Html::parse_fragment(&html);
    for selector in [
        "button.if-button--create.if-button--ghost.if-button--sm",
        "button.if-button--create.if-button--ghost-primary.if-button--md",
    ] {
        assert_eq!(
            doc.select(&scraper::Selector::parse(selector).unwrap())
                .count(),
            1,
            "{html}"
        );
    }
}

#[test]
fn menu_controls_do_not_submit_an_enclosing_form() {
    fn fixture() -> Element {
        inputforge_app::ui::use_provider();
        rsx! { form { MenuRoot { MenuTrigger { "Actions" } MenuItems { MenuItem { "Rename" } } } } }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert_eq!(html.matches("type=\"button\"").count(), 2, "{html}");
}

#[derive(Default)]
struct RecordingDocument(std::cell::RefCell<Vec<String>>);
impl document::Document for RecordingDocument {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}
use crate::components::test_events as events;

#[test]
fn visible_group_actions_keep_keyboard_navigation_and_restore_trigger_after_activation() {
    use super::{MenuGroup, MenuTriggerPresentation};
    use std::rc::Rc;
    fn fixture() -> Element {
        inputforge_app::ui::use_provider();
        rsx! {
            MenuRoot {
                MenuTrigger { presentation: MenuTriggerPresentation::Icon, aria_label: "Mapping actions", "⋯" }
                MenuItems {
                    MenuGroup { label: "Duplicate to mode",
                        MenuItem { "Duplicate to mode Flight" }
                        MenuItem { compact: true, danger: true, disabled: true, title: "Current mode", "Duplicate to mode Combat" }
                    }
                }
            }
        }
    }
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let doc = Rc::new(RecordingDocument::default());
    let mut dom = VirtualDom::new(fixture);
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    events::event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    let fragment = scraper::Html::parse_fragment(&html);
    assert_eq!(
        fragment
            .select(&scraper::Selector::parse("button button").unwrap())
            .count(),
        0,
        "{html}"
    );
    assert_eq!(
        fragment
            .select(
                &scraper::Selector::parse(
                    "[role=group][aria-label='Duplicate to mode'] [role=menuitem]"
                )
                .unwrap()
            )
            .count(),
        2,
        "{html}"
    );
    assert!(html.contains("title=\"Current mode\""), "{html}");
    for key in [Key::ArrowDown, Key::ArrowUp, Key::Home, Key::End] {
        assert!(!events::key(&dom, 0, key).default_action_enabled());
    }
    let before = doc.0.borrow().len();
    events::event(&dom, "click", 1, SerializedMouseData::default());
    assert!(
        doc.0.borrow().len() > before,
        "accepted item activation must request trigger focus"
    );
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("aria-expanded=\"false\""));
}
