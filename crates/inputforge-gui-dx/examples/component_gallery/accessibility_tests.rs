use super::gallery_root;
use dioxus::prelude::*;
use std::collections::HashSet;

#[test]
fn adoption_gallery_controls_have_persistent_names_and_unique_ids() {
    let mut dom = VirtualDom::new(gallery_root);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let document = scraper::Html::parse_document(&html);
    let selector = |value| scraper::Selector::parse(value).unwrap();
    let mut ids = HashSet::new();
    for element in document.select(&selector("[id]")) {
        assert!(
            ids.insert(element.value().attr("id").unwrap().to_owned()),
            "duplicate ID: {html}"
        );
    }
    for control in document.select(&selector("input,select")) {
        let value = control.value();
        let named = value
            .attr("aria-label")
            .is_some_and(|name| !name.trim().is_empty())
            || value.attr("aria-labelledby").is_some()
            || control
                .ancestors()
                .filter_map(scraper::ElementRef::wrap)
                .any(|parent| {
                    parent.value().name() == "label"
                        && !parent.text().collect::<String>().trim().is_empty()
                })
            || value.attr("id").is_some_and(|id| {
                document
                    .select(&selector("label[for]"))
                    .any(|label| label.value().attr("for") == Some(id))
            });
        assert!(named, "unnamed gallery control: {}", control.html());
    }
    for element in document.select(&selector(
        "[aria-describedby],[aria-labelledby],[aria-errormessage],[aria-controls]",
    )) {
        for attribute in [
            "aria-describedby",
            "aria-labelledby",
            "aria-errormessage",
            "aria-controls",
        ] {
            for id in element
                .value()
                .attr(attribute)
                .unwrap_or_default()
                .split_whitespace()
            {
                assert!(ids.contains(id), "dangling {attribute}={id}");
            }
        }
    }
}
