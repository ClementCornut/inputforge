//! One calibrated view for routing and processed preview; raw capture stays untouched.
use super::{AppState, DeviceCalibrationStore, InputCacheStore};
use crate::{
    pipeline::InputCache,
    types::{AxisPolarity, HatDirection, InputAddress, InputId},
};

#[derive(Debug)]
pub struct InputValues<'a> {
    pub raw: &'a InputCacheStore,
    calibrations: &'a DeviceCalibrationStore,
}
impl<'a> InputValues<'a> {
    #[must_use]
    pub fn new(state: &'a AppState) -> Self {
        Self::from_parts(&state.input_cache, &state.calibrations)
    }
    #[must_use]
    pub fn from_parts(raw: &'a InputCacheStore, calibrations: &'a DeviceCalibrationStore) -> Self {
        Self { raw, calibrations }
    }
}
impl InputCache for InputValues<'_> {
    fn get_axis(&self, address: &InputAddress) -> (f64, AxisPolarity) {
        let (mut value, polarity) = self.raw.get_axis(address);
        if let InputAddress::Bound {
            device,
            input: InputId::Axis { index },
        } = address
            && let Some(calibration) = self.calibrations.get(device, *index)
        {
            value = calibration.apply(value);
        }
        (value, polarity)
    }
    fn get_button(&self, address: &InputAddress) -> bool {
        self.raw.get_button(address)
    }
    fn get_hat(&self, address: &InputAddress) -> HatDirection {
        self.raw.get_hat(address)
    }
}
