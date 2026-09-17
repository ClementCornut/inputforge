//! Read-only Linux event-device discovery. Native codes are not binding indices.
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
