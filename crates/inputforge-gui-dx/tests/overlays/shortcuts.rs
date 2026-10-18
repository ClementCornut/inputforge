//! Real closed/open menu markup is checked by each production shortcut selector.
use super::{event, menu};
use dioxus::prelude::*;
use scraper::{Html, Selector};
use std::{collections::BTreeMap, io::Write, process::Stdio};

#[test]
fn actual_closed_menus_do_not_block_editor_rail_or_snapshot_shortcuts() {
    check(false);
}

#[test]
fn actual_open_menus_still_block_editor_rail_and_snapshot_shortcuts() {
    check(true);
}

fn check(open: bool) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(menu);
    dom.rebuild_in_place();
    if open {
        event(&dom, "click", 0, SerializedMouseData::default());
        dom.render_immediate_to_vec();
    }
    let html = Html::parse_fragment(&dioxus_ssr::render(&dom));
    let menu = html
        .select(&Selector::parse("[role=menu]").unwrap())
        .next()
        .unwrap();
    assert_eq!(menu.value().attr("aria-hidden"), (!open).then_some("true"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut failures = Vec::new();
    for script in [
        "editor_shortcuts.js",
        "rail_shortcuts.js",
        "snapshot_shortcut.js",
    ] {
        let source =
            std::fs::read_to_string(root.join("crates/inputforge-app/assets").join(script))
                .unwrap();
        let selectors: BTreeMap<_, _> = source
            .split("document.querySelector('")
            .skip(1)
            .map(|s| {
                let selector = s.split('\'').next().unwrap();
                (
                    selector,
                    html.select(&Selector::parse(selector).unwrap())
                        .next()
                        .is_some(),
                )
            })
            .collect();
        let mut child = std::process::Command::new("node")
            .arg(root.join("crates/inputforge-gui-dx/tests/overlays/shortcuts.mjs"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = serde_json::json!({"script": script, "source": source, "selectors": selectors, "blocked": open});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        if !result.status.success() {
            failures.push(format!(
                "{script} open={open}: {}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
