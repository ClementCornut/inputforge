use super::{AxisInfo, Issue, Metadata};
use evdev::raw_stream::RawDevice;
use std::{fs::File, io};

pub(super) fn read(metadata: &Metadata) -> Result<Vec<AxisInfo>, Issue> {
    // File::open is O_RDONLY. Avoid evdev::open, which first tries write access.
    let file = File::open(&metadata.node)
        .map_err(|error| Issue::new("open read-only", &metadata.node, &error))?;
    read_file(metadata, file)
}

fn read_file(metadata: &Metadata, file: File) -> Result<Vec<AxisInfo>, Issue> {
    let device = RawDevice::from_fd(file.into())
        .map_err(|error| Issue::new("query evdev metadata", &metadata.node, &error))?;
    verify(metadata, &device)?;
    device
        .get_absinfo()
        .map(|axes| {
            axes.map(|(code, axis)| AxisInfo {
                code: code.0,
                minimum: axis.minimum(),
                maximum: axis.maximum(),
                fuzz: axis.fuzz(),
                flat: axis.flat(),
                resolution: axis.resolution(),
            })
            .collect()
        })
        .map_err(|error| Issue::new("query ABS metadata", &metadata.node, &error))
}

pub(super) fn verify(metadata: &Metadata, device: &RawDevice) -> Result<(), Issue> {
    let id = device.input_id();
    let same = metadata.bus == Some(id.bus_type().0)
        && metadata.diagnostics.vendor_id == Some(id.vendor())
        && metadata.diagnostics.product_id == Some(id.product())
        && metadata.diagnostics.product_version == Some(id.version())
        && device.name() == Some(metadata.name.as_str())
        && device.physical_path().filter(|s| !s.is_empty()) == metadata.phys.as_deref()
        && device.unique_name().filter(|s| !s.is_empty()) == metadata.unique.as_deref()
        && device
            .properties()
            .iter()
            .map(|code| code.0)
            .eq(metadata.kernel_properties.iter().copied())
        && device
            .supported_events()
            .iter()
            .map(|code| code.0)
            .eq(metadata.event_types.iter().copied())
        && device
            .supported_keys()
            .into_iter()
            .flat_map(|codes| codes.iter())
            .map(|code| code.0)
            .eq(metadata.keys.iter().copied())
        && device
            .supported_absolute_axes()
            .into_iter()
            .flat_map(|codes| codes.iter())
            .map(|code| code.0)
            .eq(metadata.abs_axes.iter().copied())
        && device
            .supported_relative_axes()
            .into_iter()
            .flat_map(|codes| codes.iter())
            .map(|code| code.0)
            .eq(metadata.rel_axes.iter().copied());
    if !same {
        return Err(Issue::new(
            "verify event node",
            &metadata.node,
            &io::Error::new(
                io::ErrorKind::InvalidData,
                "device changed during discovery; rescan",
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{File, Metadata, read, read_file};
    use std::{fs, os::fd::AsRawFd};

    #[test]
    fn ioctl_failure_closes_owned_descriptor_and_keeps_operation_and_errno() {
        let fixture = tempfile::NamedTempFile::new().expect("regular file fixture");
        let file = File::open(fixture.path()).expect("read-only fixture");
        let fd_path = format!("/proc/self/fd/{}", file.as_raw_fd());
        let metadata = Metadata {
            node: fixture.path().into(),
            ..Metadata::default()
        };
        let issue = read_file(&metadata, file).expect_err("regular file is not evdev");
        assert_eq!(issue.operation, "query evdev metadata");
        assert_eq!(issue.raw_os_error, Some(25));
        assert_ne!(fs::read_link(fd_path).ok().as_deref(), Some(fixture.path()));
    }

    #[test]
    fn missing_node_is_an_open_failure() {
        let directory = tempfile::tempdir().expect("fixture directory");
        let metadata = Metadata {
            node: directory.path().join("missing"),
            ..Metadata::default()
        };
        let issue = read(&metadata).expect_err("missing event node");
        assert_eq!(issue.operation, "open read-only");
        assert_eq!(issue.raw_os_error, Some(2));
    }
}
