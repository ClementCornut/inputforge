mod report;
use report::Report;

use inputforge_core::device::evdev::Capture;
use std::{
    io,
    io::Write,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(super) fn run(
    capture: &mut Capture,
    duration: Duration,
    writer: &mut impl Write,
) -> Result<()> {
    let mut report = Report::default();
    let result = collect(capture, duration, &mut report);
    if result.is_err() {
        report.invalidate();
    }
    finish(
        result,
        || capture.release().map_err(Into::into),
        || report.write(writer),
    )
}

fn collect(capture: &mut Capture, duration: Duration, report: &mut Report) -> Result<()> {
    let deadline = Instant::now() + duration;
    let mut updates = Vec::new();
    while Instant::now() < deadline {
        let status = capture.poll_stream(&mut updates)?;
        for update in updates.drain(..) {
            report.observe(update);
        }
        report.set_ready(status.ready);
        if !status.pending {
            // Idle polls remain responsive to lifecycle changes without a busy loop.
            thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(())
}

// Release is unconditional and precedes both error formatting and output.
fn finish(
    result: Result<()>,
    release: impl FnOnce() -> Result<()>,
    report: impl FnOnce() -> io::Result<()>,
) -> Result<()> {
    let release = release();
    let output = report();
    let mut failures = Vec::new();
    if let Err(error) = result {
        failures.push(format!("stream: {error}"));
    }
    if let Err(error) = release {
        failures.push(format!("release: {error}"));
    }
    if let Err(error) = output {
        failures.push(format!("report: {error}"));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(failures.join("; ")).into())
    }
}

#[cfg(test)]
mod tests;
