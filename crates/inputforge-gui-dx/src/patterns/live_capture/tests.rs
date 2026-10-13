use super::*;
#[test]
fn capture_prompt_matches_shared_binding_ui_copy() {
    assert_eq!(CAPTURE_PROMPT, "Press an input\u{2026}");
}
#[test]
fn rebind_composite_class_combines_listening_and_unbound_modifiers() {
    let class = rebind_composite_class(&InputAddress::Unbound, true);
    assert!(class.contains("if-rebind-composite--listening"));
    assert!(class.contains("if-rebind-composite--unbound"));
}
