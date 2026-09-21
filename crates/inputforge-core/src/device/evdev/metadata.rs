use super::{Issue, Metadata, Ownership, UinputStatus};
use crate::device::linux_bitmap;
use std::{collections::BTreeSet, fs, io, os::unix::fs::MetadataExt, path::Path};

pub(super) fn read(device: &udev::Device) -> (Metadata, Vec<Issue>) {
    let mut metadata = Metadata {
        node: device.devnode().map_or_else(
            || Path::new("/dev/input").join(device.sysname()),
            Path::to_path_buf,
        ),
        ..Metadata::default()
    };
    let mut issues = Vec::new();
    for property in device.properties() {
        let name = property.name().to_string_lossy();
        if name.starts_with("ID_INPUT")
            || matches!(
                name.as_ref(),
                "ID_SERIAL_SHORT"
                    | "ID_PATH"
                    | "ID_USB_INTERFACE_NUM"
                    | "ID_SEAT"
                    | "TAGS"
                    | "CURRENT_TAGS"
            )
        {
            if let Some(value) = property.value().to_str() {
                metadata.properties.insert(name.into_owned(), value.into());
            } else {
                issues.push(Issue::new(
                    "udev property",
                    device.syspath(),
                    &io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 property"),
                ));
            }
        }
    }
    let parent = device.syspath().parent().unwrap_or(device.syspath());
    read_input(parent, metadata, issues)
}

fn read_input(
    parent: &Path,
    mut metadata: Metadata,
    mut issues: Vec<Issue>,
) -> (Metadata, Vec<Issue>) {
    metadata.name = required_text(parent, "name", &mut issues).unwrap_or_default();
    metadata.phys = optional_text(parent, "phys", &mut issues);
    metadata.unique = optional_text(parent, "uniq", &mut issues);
    metadata.stable_path = metadata.properties.get("ID_PATH").cloned();
    metadata.interface = metadata.properties.get("ID_USB_INTERFACE_NUM").cloned();
    metadata.bus = hex_attribute(parent, "id/bustype", &mut issues);
    metadata.diagnostics.vendor_id = hex_attribute(parent, "id/vendor", &mut issues);
    metadata.diagnostics.product_id = hex_attribute(parent, "id/product", &mut issues);
    metadata.diagnostics.product_version = hex_attribute(parent, "id/version", &mut issues);
    metadata.diagnostics.serial = metadata
        .properties
        .get("ID_SERIAL_SHORT")
        .cloned()
        .or_else(|| metadata.unique.clone());
    metadata.kernel_properties = capabilities(parent, "properties", &mut issues);
    metadata.event_types = capabilities(parent, "capabilities/ev", &mut issues);
    metadata.keys = capabilities(parent, "capabilities/key", &mut issues);
    metadata.abs_axes = capabilities(parent, "capabilities/abs", &mut issues);
    metadata.rel_axes = capabilities(parent, "capabilities/rel", &mut issues);
    match fs::read_dir(parent) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let name = entry.file_name();
                        let name = name.to_string_lossy();
                        if name
                            .strip_prefix("js")
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                        {
                            metadata.joydev.push(format!("/dev/input/{name}"));
                        }
                    }
                    Err(error) => issues.push(Issue::new("list input handlers", parent, &error)),
                }
            }
        }
        Err(error) => issues.push(Issue::new("list input handlers", parent, &error)),
    }
    metadata.joydev.sort();
    match ownership(&metadata.node) {
        Ok(value) => metadata.ownership = Some(value),
        Err(error) => issues.push(Issue::new("stat event node", &metadata.node, &error)),
    }
    (metadata, issues)
}

fn text(parent: &Path, name: &str) -> io::Result<String> {
    fs::read_to_string(parent.join(name)).map(|mut value| {
        // Sysfs adds one newline; whitespace inside the kernel string is data.
        if value.ends_with('\n') {
            value.pop();
        }
        value
    })
}
fn required_text(parent: &Path, name: &str, issues: &mut Vec<Issue>) -> Option<String> {
    match text(parent, name) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(Issue::new("read sysfs", parent.join(name), &error));
            None
        }
    }
}
fn optional_text(parent: &Path, name: &str, issues: &mut Vec<Issue>) -> Option<String> {
    match text(parent, name) {
        Ok(value) => (!value.is_empty()).then_some(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            issues.push(Issue::new("read sysfs", parent.join(name), &error));
            None
        }
    }
}
fn hex_attribute(parent: &Path, name: &str, issues: &mut Vec<Issue>) -> Option<u16> {
    let value = required_text(parent, name, issues)?;
    match u16::from_str_radix(&value, 16) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(Issue::new(
                "parse sysfs ID",
                parent.join(name),
                &io::Error::new(io::ErrorKind::InvalidData, error),
            ));
            None
        }
    }
}
fn capabilities(parent: &Path, name: &str, issues: &mut Vec<Issue>) -> BTreeSet<u16> {
    let Some(value) = required_text(parent, name, issues) else {
        return BTreeSet::new();
    };
    match linux_bitmap::parse(&value, usize::BITS) {
        Ok(codes) => codes,
        Err(error) => {
            issues.push(Issue::new("parse capabilities", parent.join(name), &error));
            BTreeSet::new()
        }
    }
}
fn ownership(path: &Path) -> io::Result<Ownership> {
    fs::metadata(path).map(|m| Ownership {
        uid: m.uid(),
        gid: m.gid(),
        mode: m.mode(),
    })
}
pub(super) fn uinput_status() -> UinputStatus {
    let path = Path::new("/dev/uinput");
    match ownership(path) {
        Ok(value) => UinputStatus {
            ownership: Some(value),
            issue: None,
        },
        Err(error) => UinputStatus {
            ownership: None,
            issue: Some(Issue::new("stat uinput", path, &error)),
        },
    }
}

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;
