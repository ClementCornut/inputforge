// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::device::DeviceId;

/// Fully qualified address of a physical input.
///
/// `Bound` is the normal case (a real device + input). `Unbound` is the
/// explicit "no binding selected yet" state used by stages added from the
/// palette before the user has chosen an input. Previously this was encoded
/// as a sentinel `Bound { device: DeviceId(""), input: ... }` which silently
/// rendered as `Btn 1` and confused users; the enum makes the state explicit
/// at the type level.
///
/// Note: deliberately does NOT implement `Default`. Choosing between
/// `Bound`-with-placeholder and `Unbound` would re-introduce the original
/// silent-default bug in a different shape; call sites must be explicit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputAddress {
    Bound { device: DeviceId, input: InputId },
    Unbound,
}

impl InputAddress {
    /// Returns `true` when this address has no binding selected yet.
    #[must_use]
    pub const fn is_unbound(&self) -> bool {
        matches!(self, Self::Unbound)
    }

    /// Returns `true` when this address points at a real device + input.
    #[must_use]
    pub const fn is_bound(&self) -> bool {
        matches!(self, Self::Bound { .. })
    }

    /// Returns the bound device, or `None` if the address is `Unbound`.
    #[must_use]
    pub const fn device(&self) -> Option<&DeviceId> {
        match self {
            Self::Bound { device, .. } => Some(device),
            Self::Unbound => None,
        }
    }

    /// Returns the bound input, or `None` if the address is `Unbound`.
    #[must_use]
    pub const fn input_id(&self) -> Option<&InputId> {
        match self {
            Self::Bound { input, .. } => Some(input),
            Self::Unbound => None,
        }
    }
}

// Helper structs for serialise. Cannot collapse to an enum because the
// untagged-enum-of-tables would silently match the wrong arm on round-trip.
#[derive(Serialize)]
struct BoundOnTheWire<'a> {
    device: &'a DeviceId,
    input: &'a InputId,
}

#[derive(Serialize)]
struct UnboundOnTheWire {
    // Always serialised as `true`; the deserializer rejects `false` to
    // prevent a silent round-trip into `Unbound`.
    unbound: bool,
}

impl Serialize for InputAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bound { device, input } => BoundOnTheWire { device, input }.serialize(serializer),
            Self::Unbound => UnboundOnTheWire { unbound: true }.serialize(serializer),
        }
    }
}

// Intermediate types for deserialise. The `Unbound` arm wraps a struct with
// `deny_unknown_fields` so that mixed-shape input (e.g.
// `{ unbound = true, device = "x", input = {...} }`) is rejected by the
// `Unbound` arm and falls through to the `Bound` arm. Without this, the
// untagged enum would silently match `Unbound` on any input containing the
// `unbound` key and drop the binding fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnboundFields {
    unbound: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum InputAddressOnTheWire {
    Unbound(UnboundFields),
    Bound { device: DeviceId, input: InputId },
}

impl<'de> Deserialize<'de> for InputAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match InputAddressOnTheWire::deserialize(deserializer)? {
            InputAddressOnTheWire::Unbound(UnboundFields { unbound: true }) => Ok(Self::Unbound),
            InputAddressOnTheWire::Unbound(UnboundFields { unbound: false }) => {
                Err(serde::de::Error::custom(
                    "InputAddress: `unbound = false` is not a valid encoding; \
                     use the bound shape instead",
                ))
            }
            InputAddressOnTheWire::Bound { device, input } => {
                if device.0.is_empty() {
                    return Err(serde::de::Error::custom(
                        "InputAddress: `device` must not be empty; use `unbound = true` for an unbound address",
                    ));
                }
                Ok(Self::Bound { device, input })
            }
        }
    }
}

/// Identifies a specific input on a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputId {
    Axis { index: u8 },
    Button { index: u8 },
    Hat { index: u8 },
}

/// Fully qualified address of a virtual output (vJoy device + output).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputAddress {
    pub device: u8,
    pub output: OutputId,
}

/// Identifies a specific output on a vJoy device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputId {
    Axis { id: VJoyAxis },
    Button { id: u8 },
    Hat { id: u8 },
}

/// vJoy axis identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VJoyAxis {
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    Slider0,
    Slider1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_id_axis_serde_roundtrip() {
        let id = InputId::Axis { index: 3 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_id_button_serde_roundtrip() {
        let id = InputId::Button { index: 7 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_id_hat_serde_roundtrip() {
        let id = InputId::Hat { index: 0 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn output_id_axis_serde_roundtrip() {
        let id = OutputId::Axis { id: VJoyAxis::X };
        let json = serde_json::to_string(&id).unwrap();
        let back: OutputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_address_bound_json_roundtrip() {
        let addr = InputAddress::Bound {
            device: DeviceId("guid-001".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        let j = serde_json::to_string(&addr).unwrap();
        assert_eq!(
            j,
            r#"{"device":"guid-001","input":{"type":"axis","index":2}}"#
        );
        let back: InputAddress = serde_json::from_str(&j).unwrap();
        assert_eq!(addr, back);
    }

    #[test]
    fn vjoy_axis_all_variants() {
        let axes = [
            VJoyAxis::X,
            VJoyAxis::Y,
            VJoyAxis::Z,
            VJoyAxis::Rx,
            VJoyAxis::Ry,
            VJoyAxis::Rz,
            VJoyAxis::Slider0,
            VJoyAxis::Slider1,
        ];
        assert_eq!(axes.len(), 8);
    }

    #[test]
    fn input_address_bound_toml_roundtrip() {
        let addr = InputAddress::Bound {
            device: DeviceId("guid-001".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        let toml_str = toml::to_string(&addr).unwrap();
        assert!(toml_str.contains("device = \"guid-001\""));
        assert!(
            !toml_str.contains("unbound"),
            "Bound must not emit `unbound`"
        );
        let back: InputAddress = toml::from_str(&toml_str).unwrap();
        assert_eq!(addr, back);
    }

    #[test]
    fn input_address_unbound_toml_roundtrip() {
        let addr = InputAddress::Unbound;
        let toml_str = toml::to_string(&addr).unwrap();
        assert_eq!(toml_str.trim(), "unbound = true");
        let back: InputAddress = toml::from_str(&toml_str).unwrap();
        assert_eq!(back, InputAddress::Unbound);
    }

    #[test]
    fn input_address_unbound_json_roundtrip() {
        let addr = InputAddress::Unbound;
        let j = serde_json::to_string(&addr).unwrap();
        assert_eq!(j, r#"{"unbound":true}"#);
        let back: InputAddress = serde_json::from_str(&j).unwrap();
        assert_eq!(back, InputAddress::Unbound);
    }

    #[test]
    fn input_address_legacy_bound_format_still_parses() {
        // A profile saved before this refactor.
        let legacy = r#"{"device":"guid-001","input":{"type":"button","index":3}}"#;
        let addr: InputAddress = serde_json::from_str(legacy).unwrap();
        assert!(matches!(addr, InputAddress::Bound { .. }));
    }

    #[test]
    fn input_address_empty_device_is_rejected() {
        // Empty `device` strings used to be the pre-refactor "no binding" sentinel.
        // The deserializer now rejects them explicitly so any malformed profile
        // fails fast at load time. Use `unbound = true` for the unbound encoding.
        let json = r#"{"device":"","input":{"type":"button","index":0}}"#;
        let result: Result<InputAddress, _> = serde_json::from_str(json);
        assert!(result.is_err(), "empty device must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("`device` must not be empty"),
            "error must mention empty device, got: {err}"
        );
        assert!(
            err.contains("`unbound = true`"),
            "error must hint at the canonical unbound shape, got: {err}"
        );
    }

    #[test]
    fn input_address_empty_device_is_rejected_via_toml() {
        // Symmetric coverage for TOML, the actual on-disk format.
        let toml_str = r#"
device = ""
[input]
type = "button"
index = 0
"#;
        let result: Result<InputAddress, _> = toml::from_str(toml_str);
        assert!(result.is_err(), "empty device must be rejected in TOML too");
    }

    #[test]
    fn input_address_unbound_false_is_rejected() {
        // The serializer never emits `unbound = false`. A deserializer that
        // accepted it would round-trip incorrectly. Lock in rejection.
        let toml_str = "unbound = false";
        let result: Result<InputAddress, _> = toml::from_str(toml_str);
        assert!(result.is_err(), "unbound = false must be rejected");
    }

    #[test]
    fn input_address_helpers() {
        let bound = InputAddress::Bound {
            device: DeviceId("d".to_owned()),
            input: InputId::Button { index: 0 },
        };
        assert!(bound.is_bound() && !bound.is_unbound());
        assert!(bound.device().is_some() && bound.input_id().is_some());

        let unbound = InputAddress::Unbound;
        assert!(unbound.is_unbound() && !unbound.is_bound());
        assert!(unbound.device().is_none() && unbound.input_id().is_none());
    }

    #[test]
    fn input_address_unbound_with_bound_fields_does_not_silently_drop_them() {
        // A mixed-shape input that includes both `unbound = true` and a
        // `device` / `input` block. The deserializer must not silently
        // pick `Unbound` and drop the binding; it should fall through to
        // the `Bound` arm.
        let mixed = r#"{"unbound":true,"device":"guid-001","input":{"type":"button","index":0}}"#;
        let addr: InputAddress = serde_json::from_str(mixed).unwrap();
        assert!(
            matches!(addr, InputAddress::Bound { .. }),
            "mixed-shape input must not be silently swallowed as Unbound, got {addr:?}"
        );
    }
}
