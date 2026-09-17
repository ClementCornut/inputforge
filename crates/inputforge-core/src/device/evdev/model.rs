//! Owned diagnostic values; no open device handles escape discovery.
use crate::types::{DeviceDiagnostics, DeviceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::PathBuf,
};

/// Information available before opening an event node.
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub node: PathBuf,
    pub name: String,
    pub diagnostics: DeviceDiagnostics,
    pub bus: Option<u16>,
    pub unique: Option<String>,
    pub phys: Option<String>,
    pub stable_path: Option<String>,
    pub interface: Option<String>,
    pub properties: BTreeMap<String, String>,
    pub kernel_properties: BTreeSet<u16>,
    pub event_types: BTreeSet<u16>,
    pub keys: BTreeSet<u16>,
    pub abs_axes: BTreeSet<u16>,
    pub rel_axes: BTreeSet<u16>,
    pub joydev: Vec<String>,
    pub ownership: Option<Ownership>,
}

/// File ownership is context, not proof of effective ACL access.
#[derive(Clone, Debug)]
pub struct Ownership {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
}

/// Conservative per-interface classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Controller,
    Ambiguous,
    Excluded,
}

#[derive(Clone, Debug)]
pub struct Classification {
    pub kind: Class,
    pub reasons: Vec<String>,
}

/// Identity quality describes reconnect guarantees, not capture readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityQuality {
    Serial,
    Port,
    Ambiguous,
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub id: Option<DeviceId>,
    pub quality: IdentityQuality,
}

/// The kernel's native ABS code and unmodified calibration metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisInfo {
    pub code: u16,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    NotAttempted,
    Readable,
    Failed,
}

/// A failure retains the operation and OS error for actionable diagnostics.
#[derive(Clone, Debug)]
pub struct Issue {
    pub operation: String,
    pub path: PathBuf,
    pub kind: io::ErrorKind,
    pub raw_os_error: Option<i32>,
    pub message: String,
}
impl Issue {
    pub(super) fn new(operation: &str, path: impl Into<PathBuf>, error: &io::Error) -> Self {
        Self {
            operation: operation.into(),
            path: path.into(),
            kind: error.kind(),
            raw_os_error: error.raw_os_error(),
            message: error.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    pub metadata: Metadata,
    pub classification: Classification,
    pub identity: Identity,
    pub axes: Vec<AxisInfo>,
    pub access: Access,
    pub issues: Vec<Issue>,
}

/// `/dev/uinput` is inspected with stat only, never opened.
#[derive(Clone, Debug, Default)]
pub struct UinputStatus {
    pub ownership: Option<Ownership>,
    pub issue: Option<Issue>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub devices: Vec<Device>,
    pub uinput: UinputStatus,
}
