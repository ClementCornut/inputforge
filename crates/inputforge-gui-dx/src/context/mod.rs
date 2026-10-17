//! Per-window prepared presentation signals.
pub(crate) use crate::snapshots::{
    ConfigSnapshot, DevicePanelRow, DeviceUsageSummary, LiveSnapshot, MappingSummary, MetaSnapshot,
    SettingsSnapshot,
};
use dioxus::prelude::*;
/// Per-window prepared presentation signals.
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub settings: Signal<SettingsSnapshot>,
    pub meta: Signal<MetaSnapshot>,
    pub config: Signal<ConfigSnapshot>,
    pub live: Signal<LiveSnapshot>,
}

#[cfg(test)]
pub(crate) use crate::snapshots::{
    DeviceCoverage, DeviceInputValues, GlyphFlags, OutputActivitySnapshot, VjoyOutputValues,
};
