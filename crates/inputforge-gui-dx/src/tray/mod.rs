//! Tray bridge — observes `dioxus-desktop`'s forwarded muda events via
//! `Config::with_custom_event_handler`, routes through a bounded
//! `tokio::sync::mpsc`, and dispatches in a Dioxus task.

pub(crate) mod action;
