use std::io::Write;

pub(super) fn write_changes(
    capture: &inputforge_core::device::evdev::Capture,
    writer: &mut impl Write,
    previous: &mut String,
) -> std::io::Result<()> {
    write_state(capture.devices(), capture.captured(), writer, previous)
}

pub(super) fn write_state(
    devices: &[inputforge_core::device::evdev::Device],
    captured: &[inputforge_core::types::DeviceId],
    writer: &mut impl Write,
    previous: &mut String,
) -> std::io::Result<()> {
    let current = render_state(devices, captured);
    if *previous != current {
        writer.write_all(current.as_bytes())?;
        writer.flush()?;
        *previous = current;
    }
    Ok(())
}

#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "Escape device-controlled names and paths in terminal output"
)]
pub(super) fn render_state(
    devices: &[inputforge_core::device::evdev::Device],
    captured: &[inputforge_core::types::DeviceId],
) -> String {
    let mut lines = vec![format!("inventory: {}", devices.len())];
    for device in devices {
        lines.push(format!(
            "  id={:?} identity={:?} class={:?} access={:?} name={:?} node={:?}",
            device.identity.id.as_ref().map(|id| &id.0),
            device.identity.quality,
            device.classification.kind,
            device.access,
            device.metadata.name,
            device.metadata.node
        ));
        lines.extend(
            device
                .classification
                .reasons
                .iter()
                .map(|reason| format!("    reason={reason:?}")),
        );
        lines.extend(device.issues.iter().map(|issue| {
            format!(
                "    issue operation={:?} path={:?} kind={:?} errno={:?} message={:?}",
                issue.operation, issue.path, issue.kind, issue.raw_os_error, issue.message
            )
        }));
    }
    lines.push(format!(
        "captured: {:?}",
        captured.iter().map(|id| id.0.as_str()).collect::<Vec<_>>()
    ));
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
