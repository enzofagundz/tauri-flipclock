#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    Pinned,
    Windowed,
}

/// Pinned mode needs a compositor that provides layer-shell; everything else is
/// an ordinary windowed clock.
pub fn resolve_window_mode(pinned: bool, layer_shell_available: bool) -> WindowMode {
    if pinned && layer_shell_available {
        WindowMode::Pinned
    } else {
        WindowMode::Windowed
    }
}
