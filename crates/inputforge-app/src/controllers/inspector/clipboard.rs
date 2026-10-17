//! Retain the native clipboard owner for the editor lifetime, including closed panels.
use anyhow::Result;
#[derive(Default)]
pub(super) struct Clipboard(Option<arboard::Clipboard>);
impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clipboard")
            .field("initialized", &self.0.is_some())
            .finish()
    }
}
impl Clipboard {
    pub(super) fn write(&mut self, text: &str) -> Result<()> {
        write_with(
            &mut self.0,
            text,
            || Ok(arboard::Clipboard::new()?),
            |c, text| Ok(c.set_text(text.to_owned())?),
        )
    }
}
fn write_with<T>(
    slot: &mut Option<T>,
    text: &str,
    open: impl FnOnce() -> Result<T>,
    write: impl FnOnce(&mut T, &str) -> Result<()>,
) -> Result<()> {
    if slot.is_none() {
        *slot = Some(open()?);
    }
    write(slot.as_mut().expect("clipboard initialized"), text)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    struct Native(Rc<Cell<usize>>);
    impl Drop for Native {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    #[test]
    fn exactly_once_write_retains_owner_until_editor_drop_and_reuses_it() {
        let drops = Rc::new(Cell::new(0));
        let mut slot = None;
        let writes = Cell::new(0);
        write_with(
            &mut slot,
            "Report",
            || Ok(Native(Rc::clone(&drops))),
            |_, text| {
                assert_eq!(text, "Report");
                writes.set(writes.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(writes.get(), 1);
        assert_eq!(drops.get(), 0);
        write_with(
            &mut slot,
            "Second",
            || panic!("reopened owner"),
            |_, _| {
                writes.set(writes.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(writes.get(), 2);
        drop(slot);
        assert_eq!(drops.get(), 1);
    }
    #[test]
    fn open_and_write_failures_are_returned_without_retry_or_lost_owner() {
        let mut slot: Option<()> = None;
        assert!(
            write_with(
                &mut slot,
                "Report",
                || anyhow::bail!("unavailable"),
                |(), _| panic!("write after failed open")
            )
            .is_err()
        );
        assert!(slot.is_none());
        assert!(
            write_with(
                &mut slot,
                "Report",
                || Ok(()),
                |(), _| anyhow::bail!("denied")
            )
            .is_err()
        );
        assert!(slot.is_some());
        write_with(
            &mut slot,
            "Retry",
            || panic!("reopened after write failure"),
            |(), _| Ok(()),
        )
        .unwrap();
    }
}
