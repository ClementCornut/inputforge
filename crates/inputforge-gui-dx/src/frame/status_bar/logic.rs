//! Pure helpers for the F7 status bar. No Dioxus runtime dependency.
// Rust guideline compliant 2026-04-29

use std::path::Path;

use inputforge_core::state::DeviceState;

/// "N/M devices", count connected vs total. Stable label for the middle slot.
#[allow(dead_code, reason = "consumed by StatusBar component in Task 20")]
pub(crate) fn device_count_label(devices: &[DeviceState]) -> String {
    let connected = devices.iter().filter(|d| d.connected).count();
    format!("{}/{} devices", connected, devices.len())
}

/// Optional warning-count badge label. `None` when zero (slot collapses).
#[allow(dead_code, reason = "consumed by StatusBar component in Task 20")]
pub(crate) fn warning_count_label(warnings: usize) -> Option<String> {
    match warnings {
        0 => None,
        1 => Some("1 warning".to_owned()),
        n => Some(format!("{n} warnings")),
    }
}

/// Truncate a path with a middle ellipsis preserving the filename.
///
/// - If `path.display().to_string()` is ≤ `max_chars`, return as-is.
/// - Otherwise: keep the leading characters and the final filename
///   component, joined by `…` (U+2026), trimmed to fit within `max_chars`.
/// - If `max_chars` is too small to fit even the filename + ellipsis,
///   returns the trailing `max_chars` characters of the file display.
///
/// `max_chars = 64` is the F7 default.
#[allow(dead_code, reason = "consumed by StatusBar component in Task 20")]
pub(crate) fn truncate_path(path: &Path, max_chars: usize) -> String {
    const ELLIPSIS: char = '…';
    let s = path.display().to_string();
    if s.chars().count() <= max_chars {
        return s;
    }

    // Keep the trailing path segment in full when possible.
    let filename = path
        .file_name()
        .map_or_else(|| s.clone(), |os| os.to_string_lossy().into_owned());
    let filename_len = filename.chars().count();

    // If even filename + ellipsis doesn't fit, return trailing slice.
    if filename_len + 1 >= max_chars {
        return s
            .chars()
            .rev()
            .take(max_chars)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }

    let head_budget = max_chars.saturating_sub(filename_len + 1);
    let head: String = s.chars().take(head_budget).collect();
    format!("{head}{ELLIPSIS}{filename}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use inputforge_core::types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo};

    use super::*;

    fn dev(id: &str, connected: bool) -> DeviceState {
        DeviceState {
            info: DeviceInfo {
                id: DeviceId(id.to_owned()),
                name: id.to_owned(),
                axes: 1,
                buttons: 0,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Unipolar],
            },
            connected,
            diagnostics: DeviceDiagnostics::default(),
        }
    }

    #[test]
    fn device_count_label_zero_zero() {
        assert_eq!(device_count_label(&[]), "0/0 devices");
    }

    #[test]
    fn device_count_label_partial() {
        assert_eq!(
            device_count_label(&[dev("a", true), dev("b", false), dev("c", true)]),
            "2/3 devices"
        );
    }

    #[test]
    fn warning_count_label_zero_is_none() {
        assert_eq!(warning_count_label(0), None);
    }

    #[test]
    fn warning_count_label_one_is_singular() {
        assert_eq!(warning_count_label(1), Some("1 warning".to_owned()));
    }

    #[test]
    fn warning_count_label_many_is_plural() {
        assert_eq!(warning_count_label(7), Some("7 warnings".to_owned()));
    }

    #[test]
    fn truncate_path_shorter_than_max_returns_as_is() {
        let p = PathBuf::from("/short.toml");
        assert_eq!(truncate_path(&p, 64), "/short.toml");
    }

    #[test]
    fn truncate_path_uses_ellipsis_and_keeps_filename() {
        let p = PathBuf::from("/long/path/to/some/deep/profile/profile.toml");
        let got = truncate_path(&p, 30);
        assert!(got.ends_with("profile.toml"), "got: {got}");
        assert!(got.contains('…'), "got: {got}");
        assert!(got.chars().count() <= 30, "got: {got}");
    }

    #[test]
    fn truncate_path_filename_longer_than_max_returns_tail() {
        let p = PathBuf::from("/a/very-long-filename-that-exceeds-budget.toml");
        let got = truncate_path(&p, 10);
        assert_eq!(got.chars().count(), 10);
    }
}
