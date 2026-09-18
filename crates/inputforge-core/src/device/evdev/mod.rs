//! Linux controller discovery, opt-in exclusive capture and native event streaming.
//!
//! [`discover`] remains read-only and retains no descriptors. [`Capture`] requires
//! explicit selection to grab devices and regular polling to detect their loss.
//! Native event codes are not positional binding indices. Neither API creates outputs.
mod classification;
mod identity;
mod model;

use classification::classify;
use identity::{identify, mark_collisions};
pub use model::{
    Access, AxisInfo, Class, Classification, Device, Identity, IdentityQuality, Issue, Metadata,
    Ownership, Report, UinputStatus,
};

#[cfg(test)]
mod classification_tests;
#[cfg(test)]
mod identity_tests;

mod bitmap;
mod discovery;
#[cfg(test)]
mod discovery_tests;

mod metadata;
mod probe;
pub use discovery::discover;

mod capture;
#[doc(inline)]
pub use capture::{
    Capture, CaptureError, NativeChange, NativeControl, NativeHat, NativeState, SnapshotKind,
    StreamStatus, StreamUpdate,
};
