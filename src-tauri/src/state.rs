//! Application state shared by the IPC commands and the clock thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Local};
use flipclock_core::geometry::{window_size, Size};
use flipclock_core::mode::{resolve_window_mode, WindowMode};
use flipclock_core::reading::{format_reading, Reading};
use flipclock_core::settings::{
    load_settings, save_settings as write_settings, validate_settings_payload, Settings,
};
use serde::Serialize;
use serde_json::Value;

/// Name of the settings file inside the app config directory.
pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Serialize)]
pub struct Bootstrap {
    pub settings: Settings,
    pub reading: Reading,
    pub layer_shell_available: bool,
    pub window_mode: WindowMode,
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub settings: Settings,
    pub restart_required: bool,
}

pub struct AppState {
    settings_path: PathBuf,
    settings: Mutex<Settings>,
    fullscreen: AtomicBool,
    layer_shell_available: AtomicBool,
    window_mode: Mutex<WindowMode>,
}

impl AppState {
    /// Loads settings from disk (defaults on any failure) and resolves the mode.
    pub fn new(settings_path: impl Into<PathBuf>, layer_shell_available: bool) -> Self {
        let settings_path = settings_path.into();
        let settings = load_settings(&settings_path);
        let window_mode = resolve_window_mode(settings.pinned, layer_shell_available);
        Self {
            settings_path,
            settings: Mutex::new(settings),
            fullscreen: AtomicBool::new(false),
            layer_shell_available: AtomicBool::new(layer_shell_available),
            window_mode: Mutex::new(window_mode),
        }
    }

    fn lock_settings(&self) -> MutexGuard<'_, Settings> {
        self.settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_mode(&self) -> MutexGuard<'_, WindowMode> {
        self.window_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn settings(&self) -> Settings {
        self.lock_settings().clone()
    }

    pub fn window_mode(&self) -> WindowMode {
        *self.lock_mode()
    }

    /// Drops a requested pinned mode when layer-shell setup fails at startup.
    ///
    /// Layer-shell is reported unavailable afterwards so the frontend shows the
    /// pinned-unavailable notice instead of offering a mode that cannot work.
    pub fn force_windowed(&self) {
        *self.lock_mode() = WindowMode::Windowed;
        self.layer_shell_available.store(false, Ordering::Release);
    }

    /// Geometry for the current settings; also used to resize after a save.
    pub fn window_size(&self) -> Size {
        let settings = self.settings();
        window_size(settings.scale, settings.show_seconds, settings.show_date)
    }

    /// Payload for the first render, with the initial reading snapped.
    pub fn bootstrap(&self, now: DateTime<Local>) -> Bootstrap {
        let settings = self.settings();
        Bootstrap {
            reading: format_reading(now, &settings, false),
            settings,
            layer_shell_available: self.layer_shell_available.load(Ordering::Acquire),
            window_mode: self.window_mode(),
        }
    }

    /// Validates and persists an IPC payload, then swaps the in-memory settings.
    ///
    /// Nothing is written when validation fails; a failed write leaves the old
    /// in-memory settings in place as well.
    pub fn save(&self, payload: &Value) -> Result<SaveResult, String> {
        let json = serde_json::to_string(payload)
            .map_err(|error| format!("cannot serialize settings payload: {error}"))?;
        let incoming = validate_settings_payload(&json).map_err(|error| error.to_string())?;

        let previous = self.settings();
        if let Err(error) = write_settings(&self.settings_path, &incoming) {
            eprintln!(
                "flipclock: cannot save settings at {}: {error}",
                self.settings_path.display()
            );
            return Err(format!("cannot save settings: {error}"));
        }
        *self.lock_settings() = incoming.clone();

        Ok(SaveResult {
            restart_required: previous.pinned != incoming.pinned,
            settings: incoming,
        })
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen.load(Ordering::Acquire)
    }

    /// Stores the requested fullscreen state and returns the resulting one.
    pub fn set_fullscreen(&self, enabled: bool) -> bool {
        self.fullscreen.store(enabled, Ordering::Release);
        enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipclock_core::mode::WindowMode;
    use flipclock_core::settings::{Corner, Scale};
    use serde_json::{json, Value};

    fn temp_state(layer_shell_available: bool) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join(SETTINGS_FILE), layer_shell_available);
        (dir, state)
    }

    #[test]
    fn bootstrap_exposes_the_frontend_shape() {
        let (_dir, state) = temp_state(true);
        let bootstrap = state.bootstrap(Local::now());
        let value = serde_json::to_value(&bootstrap).unwrap();

        let object = value.as_object().unwrap();
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "layer_shell_available",
                "reading",
                "settings",
                "window_mode"
            ]
        );
        assert_eq!(value["window_mode"], json!("windowed"));
        assert_eq!(value["reading"]["animate"], json!(false));
    }

    #[test]
    fn bootstrap_resolves_pinned_mode_only_when_layer_shell_exists() {
        let (dir, _) = temp_state(true);
        let settings = Settings {
            pinned: true,
            ..Settings::default()
        };
        std::fs::write(
            dir.path().join(SETTINGS_FILE),
            serde_json::to_string(&settings).unwrap(),
        )
        .unwrap();

        let pinned = AppState::new(dir.path().join(SETTINGS_FILE), true);
        assert_eq!(pinned.window_mode(), WindowMode::Pinned);
        assert_eq!(
            serde_json::to_value(pinned.bootstrap(Local::now())).unwrap()["window_mode"],
            json!("pinned")
        );

        let fallback = AppState::new(dir.path().join(SETTINGS_FILE), false);
        assert_eq!(fallback.window_mode(), WindowMode::Windowed);
        assert_eq!(
            serde_json::to_value(fallback.bootstrap(Local::now())).unwrap()["window_mode"],
            json!("windowed")
        );
    }

    #[test]
    fn startup_fallback_reports_layer_shell_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);
        std::fs::write(
            &path,
            serde_json::to_string(&Settings {
                pinned: true,
                ..Settings::default()
            })
            .unwrap(),
        )
        .unwrap();

        let state = AppState::new(&path, true);
        assert_eq!(state.window_mode(), WindowMode::Pinned);

        state.force_windowed();

        assert_eq!(state.window_mode(), WindowMode::Windowed);
        let bootstrap = serde_json::to_value(state.bootstrap(Local::now())).unwrap();
        assert_eq!(bootstrap["layer_shell_available"], json!(false));
        assert_eq!(bootstrap["window_mode"], json!("windowed"));
    }

    #[test]
    fn save_without_pin_change_does_not_need_a_restart() {
        let (_dir, state) = temp_state(true);
        let result = state
            .save(&json!({ "format_24h": false, "show_seconds": true }))
            .unwrap();

        assert!(!result.restart_required);
        assert!(!result.settings.format_24h);
        assert!(result.settings.show_seconds);
        assert!(state.settings().show_seconds);
    }

    #[test]
    fn save_reports_a_restart_when_pin_changes() {
        let (_dir, state) = temp_state(true);
        let result = state.save(&json!({ "pinned": true })).unwrap();

        assert!(result.restart_required);
        assert!(result.settings.pinned);
    }

    #[test]
    fn invalid_payload_is_rejected_without_touching_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);
        let stored = Settings {
            pinned: true,
            corner: Corner::BottomLeft,
            scale: Scale::Large,
            ..Settings::default()
        };
        std::fs::write(&path, serde_json::to_string_pretty(&stored).unwrap()).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let state = AppState::new(&path, true);
        let error = state
            .save(&json!({ "format_24h": "not a boolean" }))
            .unwrap_err();

        assert!(error.contains("format_24h"), "unexpected error: {error}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        assert_eq!(state.settings(), stored);
    }

    #[test]
    fn save_creates_a_missing_config_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(SETTINGS_FILE);
        let state = AppState::new(&path, true);

        state.save(&json!({ "show_date": true })).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn saved_settings_reload_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);
        let state = AppState::new(&path, true);
        state
            .save(&json!({ "corner": "bottom_right", "scale": "small" }))
            .unwrap();

        let reloaded = AppState::new(&path, true).settings();
        assert_eq!(reloaded.corner, Corner::BottomRight);
        assert_eq!(reloaded.scale, Scale::Small);
    }

    #[test]
    fn window_size_follows_the_saved_display_settings() {
        let (_dir, state) = temp_state(true);
        assert_eq!(state.window_size().width, 464.0);
        assert_eq!(state.window_size().height, 168.0);

        state
            .save(&json!({ "scale": "large", "show_seconds": true, "show_date": true }))
            .unwrap();

        let expected = window_size(Scale::Large, true, true);
        assert_eq!(state.window_size(), expected);
    }

    #[test]
    fn fullscreen_reports_the_resulting_state() {
        let (_dir, state) = temp_state(true);
        assert!(!state.is_fullscreen());
        assert!(!state.set_fullscreen(false));
        assert!(!state.is_fullscreen());

        assert!(state.set_fullscreen(true));
        assert!(state.is_fullscreen());
        assert!(state.set_fullscreen(true));

        assert!(!state.set_fullscreen(false));
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn reading_serializes_with_the_frontend_field_names() {
        use chrono::{FixedOffset, TimeZone};

        let utc = FixedOffset::east_opt(0).unwrap();
        let afternoon = utc.with_ymd_and_hms(2026, 10, 30, 13, 5, 9).unwrap();
        let settings = Settings {
            format_24h: false,
            ..Settings::default()
        };
        let value: Value =
            serde_json::to_value(format_reading(afternoon, &settings, true)).unwrap();

        assert_eq!(
            value,
            json!({
                "hour": "01",
                "minute": "05",
                "second": "09",
                "date": "30/10/2026",
                "meridiem": "PM",
                "animate": true,
            })
        );

        let value: Value =
            serde_json::to_value(format_reading(afternoon, &Settings::default(), false)).unwrap();
        assert_eq!(value["hour"], json!("13"));
        assert_eq!(value["meridiem"], Value::Null);
    }

    #[test]
    fn corrupt_settings_load_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);
        std::fs::write(&path, "{ not json").unwrap();

        let state = AppState::new(&path, false);
        assert_eq!(state.settings(), Settings::default());
    }
}
