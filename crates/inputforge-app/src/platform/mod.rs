use anyhow::Result;

use inputforge_core::device::InputSource;
use inputforge_core::output::{KeyboardSink, MouseSink, OutputSink};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use self::linux as implementation;
#[cfg(target_os = "windows")]
use self::windows as implementation;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("inputforge-app supports only Windows and Linux");

pub(crate) struct PlatformBackends {
    pub(crate) input: Box<dyn InputSource>,
    pub(crate) controller: Box<dyn OutputSink>,
    pub(crate) keyboard: Box<dyn KeyboardSink>,
    pub(crate) mouse: Box<dyn MouseSink>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn create() -> Result<PlatformBackends> {
    implementation::create()
}
