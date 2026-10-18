mod fixture;
mod focus;
use dioxus::prelude::*;
use fixture::*;
#[test]
fn failed_add_retains_draft_selection_and_focus() {
    let (mut dom, f, rx) = mount();
    click(&dom, "Add mode");
    settle(&mut dom);
    input(&dom, "Approach");
    settle(&mut dom);
    drop(rx);
    enter(&dom);
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Approach"), "draft lost: {html}");
    assert!(
        html.contains("role=\"alert\""),
        "delivery error missing: {html}"
    );
    assert_eq!(*f.view.borrow().unwrap().editing_mode.peek(), "Default");
    assert_eq!(*f.focus.borrow().unwrap().peek(), None);
}
#[test]
fn failed_rename_retains_inline_draft() {
    let (mut dom, f, rx) = mount();
    menu(&mut dom);
    click(&dom, "Rename");
    settle(&mut dom);
    input(&dom, "Approach");
    settle(&mut dom);
    let focus = f.focus.borrow().unwrap().peek().clone();
    drop(rx);
    enter(&dom);
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("rename-inline"), "rename closed: {html}");
    assert!(html.contains("Approach"));
    assert!(html.contains("role=\"alert\""));
    assert_eq!(*f.focus.borrow().unwrap().peek(), focus);
}
#[test]
fn failed_delete_retains_confirmation_and_focus() {
    let (mut dom, f, rx) = mount();
    menu(&mut dom);
    click(&dom, "Delete");
    settle(&mut dom);
    let focus = f.focus.borrow().unwrap().peek().clone();
    drop(rx);
    click(&dom, "Confirm");
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Delete mode"), "confirmation closed: {html}");
    assert!(html.contains("role=\"alert\""));
    assert_eq!(*f.focus.borrow().unwrap().peek(), focus);
}
#[test]
fn stale_add_does_not_target_an_identically_named_replacement_profile() {
    let (mut dom, f, rx) = mount();
    click(&dom, "Add mode");
    settle(&mut dom);
    input(&dom, "Approach");
    settle(&mut dom);
    f.state.write().active_profile = Some(inputforge_core::profile::Profile::new(
        "P".into(),
        vec![],
        inputforge_core::mode::Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    enter(&dom);
    assert!(rx.try_recv().is_err(), "stale profile received command");
}

#[test]
fn successful_add_navigation_survives_older_metadata_before_execution() {
    let (mut dom, f, rx) = mount();
    click(&dom, "Add mode");
    settle(&mut dom);
    input(&dom, "Approach");
    settle(&mut dom);
    enter(&dom);
    settle(&mut dom);
    assert!(
        matches!(rx.try_recv().unwrap(),inputforge_core::engine::EngineCommand::AddMode {name} if name=="Approach")
    );
    for i in 0..2 {
        let mut meta = f.meta.borrow().unwrap();
        meta.write().warnings.push(format!("older tick {i}"));
        settle(&mut dom);
    }
    assert_eq!(*f.view.borrow().unwrap().editing_mode.peek(), "Approach");
}
#[test]
fn failed_menu_delivery_retains_menu_and_does_not_request_focus() {
    for label in ["Activate", "Set as default"] {
        let (mut dom, f, rx) = mount();
        menu(&mut dom);
        drop(rx);
        click(&dom, label);
        settle(&mut dom);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"menu\""));
        assert!(html.contains("role=\"alert\""));
        assert_eq!(*f.focus.borrow().unwrap().peek(), None);
    }
}

#[test]
fn adoption_rename_has_name_and_mounted_error_relationship() {
    let (mut dom, _, rx) = mount();
    menu(&mut dom);
    click(&dom, "Rename");
    settle(&mut dom);
    input(&dom, "With spaces");
    settle(&mut dom);
    drop(rx);
    enter(&dom);
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    let doc = scraper::Html::parse_fragment(&html);
    let input = doc
        .select(&scraper::Selector::parse("input").unwrap())
        .next()
        .unwrap();
    assert_eq!(
        input.value().attr("aria-label"),
        Some("Mode name"),
        "{html}"
    );
    assert_eq!(input.value().attr("aria-invalid"), Some("true"), "{html}");
    let error = input
        .value()
        .attr("aria-errormessage")
        .expect("native error association");
    assert!(!error.chars().any(char::is_whitespace));
    assert_eq!(input.value().attr("aria-describedby"), Some(error));
    assert_eq!(
        doc.select(&scraper::Selector::parse(&format!("[id='{error}']")).unwrap())
            .count(),
        1
    );
}
#[test]
fn adoption_mode_confirmation_uses_danger() {
    let (mut dom, _, _) = mount();
    menu(&mut dom);
    click(&dom, "Delete");
    settle(&mut dom);
    let html = dioxus_ssr::render(&dom);
    let doc = scraper::Html::parse_fragment(&html);
    assert!(
        doc.select(&scraper::Selector::parse("button.if-button--danger").unwrap())
            .any(|b| b.text().collect::<String>() == "Confirm"),
        "{html}"
    );
}

#[test]
fn adoption_mode_menu_controls_reference_the_mounted_menu() {
    let (mut dom, _, _) = mount();
    menu(&mut dom);
    let html = dioxus_ssr::render(&dom);
    let doc = scraper::Html::parse_fragment(&html);
    let tab = doc
        .select(&scraper::Selector::parse("[role=tab][aria-expanded=true]").unwrap())
        .next()
        .expect("menu's expanded tab");
    let id = tab
        .value()
        .attr("aria-controls")
        .expect("controlled menu ID");
    let selector = scraper::Selector::parse(&format!("[role=menu][id='{id}']")).unwrap();
    assert_eq!(doc.select(&selector).count(), 1, "{html}");
}
