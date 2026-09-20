use super::{Capture, CaptureError, transaction};
use crate::types::DeviceId;

impl Capture {
    pub(super) fn release_grabs(&mut self) -> Result<(), CaptureError> {
        let mut error: Option<CaptureError> = None;
        let mut index = self.held.len();
        while index > 0 {
            index -= 1;
            if let Err(next) = self.held[index].ungrab() {
                // Closing remains the fallback when EVIOCGRAB release fails.
                self.held.remove(index);
                if let Some(first) = &mut error {
                    first.append(next);
                } else {
                    error = Some(next);
                }
            }
        }
        error.map_or(Ok(()), Err)
    }

    pub(super) fn acquire_monitored(&mut self, ids: &[DeviceId]) -> Result<(), CaptureError> {
        self.ensure_valid()?;
        if !self.ids.is_empty() || ids.is_empty() {
            return Err(CaptureError::state(
                "acquire",
                "select controllers after releasing the current selection",
            ));
        }
        self.refresh(true)?;
        self.ensure_settled()?;
        self.open_readers();
        let mut ids = ids.to_vec();
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        ids.dedup();
        let result: Result<(), CaptureError> = (|| {
            for id in &ids {
                transaction::selected(&self.devices, id)?;
                let held = self
                    .held
                    .iter_mut()
                    .find(|h| h.info.identity.id.as_ref() == Some(id))
                    .ok_or_else(|| {
                        CaptureError::state(
                            "acquire",
                            "controller could not be opened for monitoring",
                        )
                    })?;
                transaction::verify(std::slice::from_ref(held), &self.devices)?;
                held.handle
                    .grab()
                    .map_err(|e| CaptureError::device("grab", &held.info, e))?;
                held.grabbed = true;
            }
            self.refresh(true)?;
            self.ensure_settled()?;
            for held in self.held.iter().filter(|h| h.grabbed) {
                transaction::verify(std::slice::from_ref(held), &self.devices)?;
            }
            Ok(())
        })();
        if let Err(mut error) = result {
            if let Err(cleanup) = self.release_grabs() {
                error.append(cleanup);
            }
            return Err(error);
        }
        self.ids = ids;
        Ok(())
    }
}
