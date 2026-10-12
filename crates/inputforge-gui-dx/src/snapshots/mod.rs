//! Prepared rendering data supplied by the application host.
mod config;
mod live;
mod meta;
mod settings;
#[doc(inline)]
pub use config::{
    ConfigSnapshot, DeviceCoverage, DevicePanelRow, DeviceUsageSummary, GlyphFlags, MappingSummary,
};
#[doc(inline)]
pub use live::{
    CaptureEpoch, DeviceInputValues, LiveSnapshot, OutputActivitySnapshot, VjoyOutputValues,
};
#[doc(inline)]
pub use meta::{MetaSnapshot, ProfileRowOrigin, ProfileRowView, SnapshotRowView};
#[doc(inline)]
pub use settings::SettingsSnapshot;
/// Identifier for the selected mapping: mode and input address.
pub type MappingKey = (String, inputforge_core::types::InputAddress);
/// One coherent application projection for GUI publication.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// Session, profile and warning content.
    pub meta: MetaSnapshot,
    /// Devices and the selected mapping.
    pub config: ConfigSnapshot,
    /// Sampled inputs, outputs and activity.
    pub live: LiveSnapshot,
    /// Settings and active-profile snapshot count.
    pub settings: SettingsSnapshot,
    /// Prepared routing controls.
    pub routing: crate::routing::View,
    /// Prepared controller selection, layout editing and Apply controls.
    pub controllers: crate::controllers::View,
}
