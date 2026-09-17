use std::collections::HashMap;

use sha2::{Digest, Sha256};

use super::{Device, Identity, IdentityQuality, Metadata};
use crate::types::DeviceId;

const NAMESPACE: &str = "evdev:v1:";

pub(super) fn identify(metadata: &Metadata) -> Identity {
    let Some(bus) = metadata.bus else {
        return ambiguous();
    };
    let Some(vendor) = metadata.diagnostics.vendor_id else {
        return ambiguous();
    };
    let Some(product) = metadata.diagnostics.product_id else {
        return ambiguous();
    };
    let Some(interface) = interface_discriminator(metadata) else {
        return ambiguous();
    };
    let bus = format!("{bus:04x}");
    let vendor = format!("{vendor:04x}");
    let product = format!("{product:04x}");

    if let Some(serial) = genuine_serial(metadata) {
        return derived(
            IdentityQuality::Serial,
            &["serial", &bus, &vendor, &product, interface, serial],
        );
    }

    let Some(path) = metadata
        .stable_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    else {
        return ambiguous();
    };
    let name = metadata.name.trim();
    if name.is_empty() {
        return ambiguous();
    }
    derived(
        IdentityQuality::Port,
        &["port", &bus, &vendor, &product, interface, path, name],
    )
}

pub(super) fn mark_collisions(devices: &mut [Device]) {
    let mut counts = HashMap::new();
    for id in devices
        .iter()
        .filter_map(|device| device.identity.id.as_ref())
    {
        *counts.entry(id.0.clone()).or_insert(0usize) += 1;
    }
    for device in devices {
        let collision = device
            .identity
            .id
            .as_ref()
            .is_some_and(|id| counts.get(&id.0).is_some_and(|count| *count > 1));
        if collision {
            device.identity.id = None;
            device.identity.quality = IdentityQuality::Ambiguous;
            device
                .classification
                .reasons
                .push("duplicate derived identity".into());
        }
    }
}

fn genuine_serial(metadata: &Metadata) -> Option<&str> {
    metadata
        .diagnostics
        .serial
        .as_deref()
        .and_then(genuine_value)
        .or_else(|| metadata.unique.as_deref().and_then(genuine_value))
}

fn genuine_value(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()
        && !value.eq_ignore_ascii_case("noserial")
        && !value.eq_ignore_ascii_case("no_serial"))
    .then_some(value)
}

fn interface_discriminator(metadata: &Metadata) -> Option<&str> {
    metadata
        .interface
        .as_deref()
        .filter(|value| !value.trim().is_empty())
}

fn derived(quality: IdentityQuality, components: &[&str]) -> Identity {
    let mut hasher = Sha256::new();
    for component in components {
        let bytes = component.as_bytes();
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Identity {
        id: Some(DeviceId(format!(
            "{NAMESPACE}{}",
            hex::encode(hasher.finalize())
        ))),
        quality,
    }
}

fn ambiguous() -> Identity {
    Identity {
        id: None,
        quality: IdentityQuality::Ambiguous,
    }
}
