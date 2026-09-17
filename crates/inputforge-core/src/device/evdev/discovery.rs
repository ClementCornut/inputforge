use super::{
    Access, AxisInfo, Class, Device, Issue, Metadata, Report, classify, identify, mark_collisions,
};
use std::io;

/// Enumerate Linux input interfaces and inspect eligible devices read-only.
///
/// Does not read event streams, grab devices, create outputs or change permissions.
/// Per-device failures remain in the returned report. No handles escape this call.
///
/// # Errors
/// Returns an error if libudev cannot initialize or enumerate the input subsystem.
pub fn discover() -> io::Result<Report> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("input")?;
    enumerator.match_sysname("event*")?;
    let entries = enumerator
        .scan_devices()?
        .map(|device| super::metadata::read(&device))
        .collect();
    let mut report = assemble(entries, super::probe::read);
    report.uinput = super::metadata::uinput_status();
    Ok(report)
}

pub(super) fn assemble(
    entries: Vec<(Metadata, Vec<Issue>)>,
    mut probe: impl FnMut(&Metadata) -> Result<Vec<AxisInfo>, Issue>,
) -> Report {
    let mut devices: Vec<_> = entries
        .into_iter()
        .map(|(metadata, mut issues)| {
            let mut classification = classify(&metadata);
            if issues
                .iter()
                .any(|issue| issue.operation != "stat event node")
                && classification.kind != Class::Excluded
            {
                classification.kind = Class::Ambiguous;
                classification
                    .reasons
                    .push("Incomplete metadata; rescan before considering capture".into());
            }
            let identity = identify(&metadata);
            let (access, axes) = if classification.kind == Class::Controller {
                match probe(&metadata) {
                    Ok(axes) => (Access::Readable, axes),
                    Err(issue) => {
                        issues.push(issue);
                        (Access::Failed, vec![])
                    }
                }
            } else {
                (Access::NotAttempted, vec![])
            };
            Device {
                metadata,
                classification,
                identity,
                axes,
                access,
                issues,
            }
        })
        .collect();
    devices.sort_by(|left, right| left.metadata.node.cmp(&right.metadata.node));
    mark_collisions(&mut devices);
    Report {
        devices,
        ..Report::default()
    }
}
