use evdev::InputEvent;
use std::{io, time::Instant};

pub(in crate::output::uinput) fn packet(
    mut events: &[InputEvent],
    mut write: impl FnMut(&[InputEvent]) -> io::Result<usize>,
    mut now: impl FnMut() -> Instant,
    deadline: Instant,
) -> io::Result<()> {
    // Four attempts bound interruption and short-write retries independently of the clock.
    for _ in 0..4 {
        if now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "packet write deadline",
            ));
        }
        match write(events) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(bytes) => {
                let size = size_of::<InputEvent>();
                if bytes > size_of_val(events) || bytes % size != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid native write count",
                    ));
                }
                events = &events[bytes / size..];
                if events.is_empty() {
                    return Ok(());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "packet write attempt limit",
    ))
}
