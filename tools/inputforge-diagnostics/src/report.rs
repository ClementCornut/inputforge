//! One-shot diagnostics. This path constructs no application services.
use anyhow::Context;
use inputforge_core::device::evdev::{Device, Issue, Ownership, Report};
use std::io::{self, Write};

pub(crate) fn run_with(
    discover: impl FnOnce() -> io::Result<Report>,
    mut writer: impl Write,
) -> anyhow::Result<()> {
    let report = discover().context("could not enumerate Linux input devices")?;
    render(&report, &mut writer)?;
    writer.flush()?;
    Ok(())
}

fn render(report: &Report, writer: &mut impl Write) -> io::Result<()> {
    writeln!(
        writer,
        "InputForge Linux discovery (read-only): {} event interfaces",
        report.devices.len()
    )?;
    writeln!(
        writer,
        "No event streams read; no grabs, outputs, settings or permission changes."
    )?;
    writeln!(
        writer,
        "Controller means discovery candidate; capture readiness and busy state are NOT tested."
    )?;
    writeln!(
        writer,
        "Native capability codes below are hexadecimal, not InputForge binding indices."
    )?;
    for device in &report.devices {
        render_device(device, writer)?;
    }
    writeln!(
        writer,
        "\n/dev/uinput: metadata only; write access and device creation NOT tested"
    )?;
    if let Some(owner) = &report.uinput.ownership {
        render_owner(owner, writer)?;
    }
    if let Some(issue) = &report.uinput.issue {
        render_issue(issue, writer)?;
    }
    if report.uinput.ownership.is_none() && report.uinput.issue.is_none() {
        writeln!(writer, "  Metadata unavailable")?;
    }
    Ok(())
}

#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "Escape device-controlled paths in terminal output"
)]
fn render_device(device: &Device, writer: &mut impl Write) -> io::Result<()> {
    let metadata = &device.metadata;
    writeln!(
        writer,
        "\n{:?} {:?}: {:?}",
        metadata.node, metadata.name, device.classification.kind
    )?;
    for reason in &device.classification.reasons {
        writeln!(writer, "  Reason: {reason:?}")?;
    }
    writeln!(
        writer,
        "  Identity: {:?} {:?}",
        device.identity.quality,
        device.identity.id.as_ref().map(|id| &id.0)
    )?;
    writeln!(
        writer,
        "  Access: {:?}; bus={:04x?} VID={:04x?} PID={:04x?} version={:04x?}",
        device.access,
        metadata.bus,
        metadata.diagnostics.vendor_id,
        metadata.diagnostics.product_id,
        metadata.diagnostics.product_version
    )?;
    writeln!(
        writer,
        "  Path={:?} interface={:?} phys={:?} uniq={:?} serial={:?}",
        metadata.stable_path,
        metadata.interface,
        metadata.phys,
        metadata.unique,
        metadata.diagnostics.serial
    )?;
    writeln!(writer, "  Same-parent joydev: {:?}", metadata.joydev)?;
    if let Some(owner) = &metadata.ownership {
        render_owner(owner, writer)?;
    }
    for (key, value) in &metadata.properties {
        writeln!(writer, "  {key:?}={value:?}")?;
    }
    writeln!(
        writer,
        "  EV={:x?} PROP={:x?}\n  KEY={:x?}\n  ABS={:x?} REL={:x?}",
        metadata.event_types,
        metadata.kernel_properties,
        metadata.keys,
        metadata.abs_axes,
        metadata.rel_axes
    )?;
    for x in [0x10, 0x12, 0x14, 0x16] {
        let present_x = metadata.abs_axes.contains(&x);
        let present_y = metadata.abs_axes.contains(&(x + 1));
        if present_x || present_y {
            writeln!(
                writer,
                "  Hat pair {x:02x}/{:02x}: {}",
                x + 1,
                if present_x && present_y {
                    "complete"
                } else {
                    "incomplete; no synthetic partner"
                }
            )?;
        }
    }
    for axis in &device.axes {
        writeln!(
            writer,
            "  ABS {:02x}: min={} max={} fuzz={} flat={} resolution={}",
            axis.code, axis.minimum, axis.maximum, axis.fuzz, axis.flat, axis.resolution
        )?;
    }
    for issue in &device.issues {
        render_issue(issue, writer)?;
    }
    Ok(())
}

fn render_owner(owner: &Ownership, writer: &mut impl Write) -> io::Result<()> {
    writeln!(
        writer,
        "  Node: uid={} gid={} mode={:o} character_device={} (ACL access not inferred)",
        owner.uid,
        owner.gid,
        owner.mode & 0o7777,
        owner.mode & 0o170_000 == 0o020_000
    )
}

#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "Escape device-controlled paths in terminal output"
)]
fn render_issue(issue: &Issue, writer: &mut impl Write) -> io::Result<()> {
    writeln!(
        writer,
        "  Issue: {:?} {:?}: {:?} errno={:?}: {:?}",
        issue.operation, issue.path, issue.kind, issue.raw_os_error, issue.message
    )?;
    let advice = match (issue.kind, issue.raw_os_error) {
        (io::ErrorKind::PermissionDenied, _) | (_, Some(1)) => {
            "Inspect this node with getfacl and udevadm info. Check the active local seat, uaccess tag and session ACL; a sandbox may also deny access. Do not run InputForge as root or grant broad input-group access."
        }
        (io::ErrorKind::NotFound, _) => {
            "Node or sysfs metadata is unavailable in this namespace, or the device disconnected. Compare /sys/class/input with /dev/input in the normal desktop session and rescan."
        }
        (_, Some(19)) => "Device disconnected during discovery; reconnect and rescan.",
        (io::ErrorKind::InvalidData, _) => {
            "Metadata is incomplete, inconsistent or changed during discovery; rescan and retain this diagnostic if it repeats."
        }
        _ => {
            "Retain the operation, path and errno when reporting this failure; rescan after checking the device connection."
        }
    };
    writeln!(writer, "  Next: {advice}")
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
