//! Track warning tails across appends, truncation, replacement and resets.
#[derive(Debug, Default)]
pub(super) struct Tail(Option<Vec<String>>);
impl Tail {
    pub(super) fn observe<'a>(&mut self, current: &'a [String]) -> &'a [String] {
        let Some(previous) = self.0.as_ref() else {
            self.0 = Some(current.to_vec());
            return &[];
        };
        if previous == current {
            return &[];
        }
        let overlap = if previous.starts_with(current) {
            current.len()
        } else {
            (0..=previous.len().min(current.len()))
                .rev()
                .find(|&length| previous[previous.len() - length..] == current[..length])
                .unwrap_or(0)
        };
        self.0 = Some(current.to_vec());
        &current[overlap..]
    }
}
