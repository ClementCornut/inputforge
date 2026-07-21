use anyhow::{Result, anyhow};

use super::PlatformBackends;

const UNAVAILABLE_MESSAGE: &str = "Linux input and output backends are unavailable in Slice 1; evdev and uinput arrive in later slices.";

pub(super) fn preflight() -> Result<()> {
    Err(anyhow!(UNAVAILABLE_MESSAGE))
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Slice 1 preflight exits before backend construction"
    )
)]
pub(super) fn create() -> Result<PlatformBackends> {
    Err(anyhow!(UNAVAILABLE_MESSAGE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_returns_stable_unavailable_error() {
        let error = preflight().expect_err("Linux preflight must reject startup");
        assert_eq!(error.to_string(), UNAVAILABLE_MESSAGE);
    }

    #[test]
    fn create_returns_the_same_unavailable_error() {
        let Err(error) = create() else {
            panic!("Linux must not construct placeholder backends");
        };
        assert_eq!(error.to_string(), UNAVAILABLE_MESSAGE);
    }
}
