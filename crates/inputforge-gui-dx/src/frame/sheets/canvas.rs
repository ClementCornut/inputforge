#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use std::fmt::Write as _;
use std::path::Path;

use dioxus::prelude::*;
use inputforge_core::sheet::AnchorId;
use serde::Deserialize;

use crate::frame::sheets::state::{SheetTool, SheetsState, normalized_image_point};

#[derive(Debug, Clone, Copy, Deserialize)]
struct StageRectPayload {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

const STAGE_RESIZE_LISTENER_PREFIX: &str = "__inputforgeSheetsStageResize_";

#[component]
pub(crate) fn SheetsCanvas(
    sheets: Signal<SheetsState>,
    on_import_image: EventHandler<()>,
    on_arm_capture: EventHandler<AnchorId>,
) -> Element {
    let stage_rect = use_signal(|| StageRectPayload {
        left: 0.0,
        top: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let stage_id = "if-sheets-image-stage";
    let state = sheets.read();
    let selected_template = state.selected_template().cloned();
    let selected_asset = state.selected_asset().cloned();
    let selected_asset_health = state.selected_asset_health().cloned();
    let selected_asset_missing = state.selected_asset_missing();
    let selected_anchor_id = state.selected_anchor_id.clone();
    let tool = state.tool;
    drop(state);

    let has_template = selected_template.is_some();
    let anchor_count = selected_template
        .as_ref()
        .map_or(0, |template| template.anchors.len());
    let anchor_count_label = if anchor_count == 1 {
        "1 anchor".to_owned()
    } else {
        format!("{anchor_count} anchors")
    };
    let handle_import_image = move |_| on_import_image.call(());
    let label = if has_template {
        "Template image"
    } else {
        "Import image"
    };
    let asset_id = selected_asset
        .as_ref()
        .map(|asset| asset.asset_id.as_str().to_owned())
        .unwrap_or_default();
    let asset_src = selected_asset_health
        .as_ref()
        .filter(|health| !health.missing)
        .map(|health| file_url_from_path(&health.copied_absolute_path))
        .unwrap_or_default();
    let missing_asset_entry = selected_asset_health
        .as_ref()
        .map(|health| &health.entry)
        .or(selected_asset.as_ref());
    let missing_copied_path = selected_asset_health
        .as_ref()
        .map(|health| health.copied_absolute_path.display().to_string())
        .or_else(|| missing_asset_entry.map(|asset| asset.copied_path.display().to_string()));
    let missing_original_path = missing_asset_entry
        .and_then(|asset| asset.original_import_path.as_ref())
        .map(|path| path.display().to_string());
    let current_stage_listener_key =
        stage_subscription_key(has_template, selected_asset_missing, &asset_id)
            .map(|stage_key| stage_resize_listener_key(&stage_key));
    let mut active_stage_listener_key = use_signal(|| Option::<String>::None);
    use_effect(use_reactive!(|current_stage_listener_key| {
        let previous_key = active_stage_listener_key.peek().clone();
        if previous_key == current_stage_listener_key {
            return;
        }

        if let Some(listener_key) = previous_key {
            spawn(async move {
                let _ = document::eval(&cleanup_stage_resize_listener_script(&listener_key));
            });
        }

        active_stage_listener_key.set(current_stage_listener_key.clone());

        let Some(listener_key) = current_stage_listener_key.clone() else {
            return;
        };

        let stage_id = stage_id.to_owned();
        let mut stage_rect = stage_rect;
        spawn(async move {
            let js = install_stage_resize_listener_script(&stage_id, &listener_key);
            let mut handle = document::eval(&js);
            loop {
                let Ok(rect) = handle.recv::<StageRectPayload>().await else {
                    break;
                };
                stage_rect.set(rect);
            }
        });
    }));
    use_drop(move || {
        let listener_key = active_stage_listener_key.peek().clone();
        spawn(async move {
            if let Some(listener_key) = listener_key {
                let _ = document::eval(&cleanup_stage_resize_listener_script(&listener_key));
            }
        });
    });
    let handle_stage_click = move |evt: MouseEvent| {
        let coordinates = evt.client_coordinates();
        let rect = *stage_rect.read();
        let Some(position) = normalized_image_point(
            coordinates.x,
            coordinates.y,
            rect.left,
            rect.top,
            rect.width,
            rect.height,
        ) else {
            return;
        };

        match tool {
            SheetTool::PlaceAnchor => {
                sheets.write().place_anchor(position);
            }
            SheetTool::CaptureAssign => {
                let anchor_id = sheets.write().place_anchor(position);
                on_arm_capture.call(anchor_id);
            }
            SheetTool::Select | SheetTool::ManualAssign => {}
        }
    };

    rsx! {
        main { "data-testid": "sheets-canvas",
            h2 { "{label}" }
            if !has_template {
                button {
                    "type": "button",
                    onclick: handle_import_image,
                    "Import image"
                }
            } else if let Some(template) = selected_template {
                section {
                    class: "if-sheets__canvas-panel",
                    "aria-label": "Template image canvas",
                    if selected_asset_missing {
                        div { class: "if-sheets__image-status",
                            p { "Missing image asset" }
                            if let Some(copied_path) = missing_copied_path {
                                p {
                                    span { "Copied path" }
                                    code { "{copied_path}" }
                                }
                            }
                            if let Some(original_path) = missing_original_path {
                                p {
                                    span { "Original import path" }
                                    code { "{original_path}" }
                                }
                            }
                            p { "{anchor_count_label}" }
                        }
                    } else {
                        div {
                            id: "{stage_id}",
                            class: "if-sheets__image-stage",
                            "data-testid": "sheets-image-stage",
                            "data-asset-id": "{asset_id}",
                            onclick: handle_stage_click,
                            img {
                                class: "if-sheets__image",
                                src: "{asset_src}",
                                alt: "{template.display_name}",
                            }
                            for anchor in template.anchors {
                                {
                                    let anchor_id = anchor.anchor_id.clone();
                                    let selected = selected_anchor_id.as_ref() == Some(&anchor_id);
                                    let left = anchor.position.x * 100.0;
                                    let top = anchor.position.y * 100.0;
                                    rsx! {
                                        button {
                                            "type": "button",
                                            class: "if-sheets__anchor",
                                            "data-anchor-id": "{anchor_id}",
                                            "aria-pressed": if selected { "true" } else { "false" },
                                            style: "left:{left}%;top:{top}%;",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                sheets.write().selected_anchor_id = Some(anchor_id.clone());
                                            },
                                            "{anchor.label}"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "if-sheets__image-status",
                            p { "{anchor_count_label}" }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn file_url_from_path(path: &Path) -> String {
    let path = encode_file_url_path(&path.to_string_lossy().replace('\\', "/"));

    if path.starts_with("//") {
        format!("file:{path}")
    } else if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

pub(super) fn stage_subscription_key(
    has_template: bool,
    selected_asset_missing: bool,
    asset_id: &str,
) -> Option<String> {
    (has_template && !selected_asset_missing && !asset_id.is_empty()).then(|| asset_id.to_owned())
}

fn stage_resize_listener_key(stage_key: &str) -> String {
    format!("{STAGE_RESIZE_LISTENER_PREFIX}{stage_key}")
}

pub(super) fn install_stage_resize_listener_script(stage_id: &str, listener_key: &str) -> String {
    format!(
        r"
        var stage = document.getElementById({stage_id:?});
        if (!stage) return;
        var listenerKey = {listener_key:?};
        var existing = window[listenerKey];
        if (existing) {{
            window.removeEventListener('resize', existing.sendRect || existing);
            if (existing.observer) {{
                existing.observer.disconnect();
            }}
            delete window[listenerKey];
        }}
        var sendRect = function() {{
            var r = stage.getBoundingClientRect();
            dioxus.send({{
                left: r.left,
                top: r.top,
                width: r.width,
                height: r.height
            }});
        }};
        var observer = new ResizeObserver(sendRect);
        observer.observe(stage);
        window[listenerKey] = {{
            sendRect: sendRect,
            observer: observer
        }};
        sendRect();
        window.addEventListener('resize', sendRect, {{ passive: true }});
        "
    )
}

pub(super) fn cleanup_stage_resize_listener_script(listener_key: &str) -> String {
    format!(
        "const h = window[{listener_key:?}];\
         if (h) {{\
           window.removeEventListener('resize', h.sendRect || h);\
           if (h.observer) {{\
             h.observer.disconnect();\
           }}\
           delete window[{listener_key:?}];\
         }}"
    )
}

fn encode_file_url_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());

    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }

    encoded
}
