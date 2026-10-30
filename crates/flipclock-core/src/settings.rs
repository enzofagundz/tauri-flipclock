use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub format_24h: bool,
    pub show_seconds: bool,
    pub show_date: bool,
    pub pinned: bool,
    pub corner: Corner,
    pub scale: Scale,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            format_24h: true,
            show_seconds: false,
            show_date: false,
            pinned: false,
            corner: Corner::TopRight,
            scale: Scale::Medium,
        }
    }
}

#[derive(Debug)]
pub struct SettingsError(pub String);

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SettingsError {}

/// Lenient parse for the stored settings file: anything unusable falls back to
/// the default for that field instead of rejecting the whole file.
pub fn settings_from_json(json: &str) -> Settings {
    parse_lenient(json).unwrap_or_default()
}

fn parse_lenient(json: &str) -> Option<Settings> {
    let map = match serde_json::from_str::<Value>(json) {
        Ok(Value::Object(map)) => map,
        _ => return None,
    };

    let defaults = Settings::default();
    Some(Settings {
        version: version_from(&map).unwrap_or(defaults.version),
        format_24h: bool_from(&map, "format_24h").unwrap_or(defaults.format_24h),
        show_seconds: bool_from(&map, "show_seconds").unwrap_or(defaults.show_seconds),
        show_date: bool_from(&map, "show_date").unwrap_or(defaults.show_date),
        pinned: bool_from(&map, "pinned").unwrap_or(defaults.pinned),
        corner: enum_from(&map, "corner").unwrap_or(defaults.corner),
        scale: enum_from(&map, "scale").unwrap_or(defaults.scale),
    })
}

/// Strict parse for an IPC payload: the JSON must be an object and every known
/// field present must have the right type.
pub fn validate_settings_payload(json: &str) -> Result<Settings, SettingsError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|error| SettingsError(format!("invalid settings JSON: {error}")))?;
    let Value::Object(map) = value else {
        return Err(SettingsError("settings payload must be a JSON object".to_string()));
    };

    let defaults = Settings::default();
    let version = match map.get("version") {
        None => defaults.version,
        Some(value) => value
            .as_u64()
            .filter(|version| *version >= 1)
            .and_then(|version| u32::try_from(version).ok())
            .ok_or_else(|| SettingsError("`version` must be an integer >= 1".to_string()))?,
    };

    Ok(Settings {
        version,
        format_24h: strict_bool(&map, "format_24h", defaults.format_24h)?,
        show_seconds: strict_bool(&map, "show_seconds", defaults.show_seconds)?,
        show_date: strict_bool(&map, "show_date", defaults.show_date)?,
        pinned: strict_bool(&map, "pinned", defaults.pinned)?,
        corner: strict_enum(&map, "corner")?.unwrap_or(defaults.corner),
        scale: strict_enum(&map, "scale")?.unwrap_or(defaults.scale),
    })
}

fn bool_from(map: &Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(Value::as_bool)
}

fn strict_bool(map: &Map<String, Value>, key: &str, default: bool) -> Result<bool, SettingsError> {
    match map.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| SettingsError(format!("`{key}` must be a boolean"))),
    }
}

fn version_from(map: &Map<String, Value>) -> Option<u32> {
    let version = u32::try_from(map.get("version")?.as_u64()?).ok()?;
    (version >= 1).then_some(version)
}

fn enum_from<T: DeserializeOwned>(map: &Map<String, Value>, key: &str) -> Option<T> {
    serde_json::from_value(map.get(key)?.clone()).ok()
}

fn strict_enum<T: DeserializeOwned>(map: &Map<String, Value>, key: &str) -> Result<Option<T>, SettingsError> {
    match map.get(key) {
        None => Ok(None),
        Some(value) => {
            let parsed = serde_json::from_value(value.clone())
                .map_err(|_| SettingsError(format!("`{key}` has an unsupported value")))?;
            Ok(Some(parsed))
        }
    }
}

/// Reads settings from `path`, falling back to defaults on any failure.
pub fn load_settings(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(json) => match parse_lenient(&json) {
            Some(settings) => settings,
            None => {
                eprintln!("flipclock: invalid settings at {}; using defaults", path.display());
                Settings::default()
            }
        },
        Err(error) => {
            eprintln!("flipclock: cannot read settings at {}: {error}; using defaults", path.display());
            Settings::default()
        }
    }
}

/// Writes settings through a temp file in the target directory and renames it
/// into place, so a reader never sees a partial file.
pub fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let temp_path = temp_path_for(path);
    fs::write(&temp_path, json)?;

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}
