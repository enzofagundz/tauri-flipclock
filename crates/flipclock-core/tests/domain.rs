use chrono::{FixedOffset, NaiveDate, TimeZone};
use flipclock_core::geometry::{window_size, Size};
use flipclock_core::mode::{resolve_window_mode, WindowMode};
use flipclock_core::reading::{format_reading, Reading};
use flipclock_core::settings::{
    load_settings, save_settings, settings_from_json, validate_settings_payload, Corner, Scale,
    Settings,
};
use flipclock_core::tick::{classify_tick, millis_until_next_second, TickKind};
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> chrono::DateTime<FixedOffset> {
    let offset = FixedOffset::east_opt(0).expect("offset");
    offset
        .from_local_datetime(
            &NaiveDate::from_ymd_opt(year, month, day)
                .expect("date")
                .and_hms_opt(hour, minute, second)
                .expect("time"),
        )
        .single()
        .expect("local")
}

fn settings_24h() -> Settings {
    Settings {
        version: 1,
        format_24h: true,
        show_seconds: false,
        show_date: false,
        pinned: false,
        corner: Corner::TopRight,
        scale: Scale::Medium,
    }
}

fn reading(hour: &str, minute: &str, second: &str, date: &str, meridiem: Option<&str>, animate: bool) -> Reading {
    Reading {
        hour: hour.to_string(),
        minute: minute.to_string(),
        second: second.to_string(),
        date: date.to_string(),
        meridiem: meridiem.map(str::to_string),
        animate,
    }
}

// --- reading: 24-hour ---

#[test]
fn formats_midnight_in_24_hour_mode() {
    let got = format_reading(at(2026, 1, 1, 0, 0, 0), &settings_24h(), true);
    assert_eq!(got, reading("00", "00", "00", "01/01/2026", None, true));
}

#[test]
fn formats_morning_time_in_24_hour_mode() {
    let got = format_reading(at(2026, 3, 5, 9, 5, 10), &settings_24h(), true);
    assert_eq!(got, reading("09", "05", "10", "05/03/2026", None, true));
}

#[test]
fn formats_noon_in_24_hour_mode() {
    let got = format_reading(at(2026, 6, 15, 12, 0, 0), &settings_24h(), true);
    assert_eq!(got, reading("12", "00", "00", "15/06/2026", None, true));
}

#[test]
fn formats_last_minute_in_24_hour_mode() {
    let got = format_reading(at(2026, 12, 31, 23, 59, 59), &settings_24h(), true);
    assert_eq!(got, reading("23", "59", "59", "31/12/2026", None, true));
}

// --- reading: 12-hour ---

#[test]
fn formats_midnight_in_12_hour_mode() {
    let mut settings = settings_24h();
    settings.format_24h = false;
    let got = format_reading(at(2026, 1, 1, 0, 0, 0), &settings, true);
    assert_eq!(got, reading("12", "00", "00", "01/01/2026", Some("AM"), true));
}

#[test]
fn formats_noon_in_12_hour_mode() {
    let mut settings = settings_24h();
    settings.format_24h = false;
    let got = format_reading(at(2026, 1, 1, 12, 0, 0), &settings, true);
    assert_eq!(got, reading("12", "00", "00", "01/01/2026", Some("PM"), true));
}

#[test]
fn formats_morning_time_in_12_hour_mode() {
    let mut settings = settings_24h();
    settings.format_24h = false;
    let got = format_reading(at(2026, 3, 5, 9, 5, 0), &settings, true);
    assert_eq!(got, reading("09", "05", "00", "05/03/2026", Some("AM"), true));
}

#[test]
fn formats_afternoon_time_in_12_hour_mode() {
    let mut settings = settings_24h();
    settings.format_24h = false;
    let got = format_reading(at(2026, 3, 5, 13, 45, 30), &settings, true);
    assert_eq!(got, reading("01", "45", "30", "05/03/2026", Some("PM"), true));
}

#[test]
fn formats_eleven_pm_in_12_hour_mode() {
    let mut settings = settings_24h();
    settings.format_24h = false;
    let got = format_reading(at(2026, 7, 9, 23, 59, 59), &settings, true);
    assert_eq!(got, reading("11", "59", "59", "09/07/2026", Some("PM"), true));
}

// --- reading: date always padded, animate passthrough ---

#[test]
fn date_uses_zero_padded_day_month_year() {
    let got = format_reading(at(2026, 2, 1, 8, 0, 0), &settings_24h(), true);
    assert_eq!(got.date, "01/02/2026");
}

#[test]
fn date_keeps_two_digit_day() {
    let got = format_reading(at(2026, 11, 30, 8, 0, 0), &settings_24h(), true);
    assert_eq!(got.date, "30/11/2026");
}

#[test]
fn animate_flag_passes_through_unchanged() {
    let still = format_reading(at(2026, 5, 5, 5, 5, 5), &settings_24h(), false);
    let moving = format_reading(at(2026, 5, 5, 5, 5, 5), &settings_24h(), true);
    assert!(!still.animate);
    assert!(moving.animate);
    assert_eq!(still.hour, moving.hour);
}

// --- tick classification ---

#[test]
fn first_tick_snaps() {
    assert_eq!(classify_tick(None, 1_000), TickKind::Snap);
}

#[test]
fn one_second_gap_is_normal() {
    assert_eq!(classify_tick(Some(1_000), 2_000), TickKind::Normal);
}

#[test]
fn two_second_gap_is_normal() {
    assert_eq!(classify_tick(Some(1_000), 3_000), TickKind::Normal);
}

#[test]
fn gap_beyond_two_seconds_snaps() {
    assert_eq!(classify_tick(Some(1_000), 3_001), TickKind::Snap);
}

#[test]
fn negative_gap_snaps() {
    assert_eq!(classify_tick(Some(2_000), 1_500), TickKind::Snap);
}

#[test]
fn short_positive_gap_is_normal() {
    assert_eq!(classify_tick(Some(1_000), 1_250), TickKind::Normal);
}

// --- millis until next second ---

#[test]
fn millis_until_next_second_from_boundary() {
    assert_eq!(millis_until_next_second(0), 1_000);
}

#[test]
fn millis_until_next_second_one_millisecond_after() {
    assert_eq!(millis_until_next_second(1), 999);
}

#[test]
fn millis_until_next_second_one_millisecond_before() {
    assert_eq!(millis_until_next_second(999), 1);
}

#[test]
fn millis_until_next_second_at_whole_second() {
    assert_eq!(millis_until_next_second(1_000), 1_000);
}

#[test]
fn millis_until_next_second_mid_second() {
    assert_eq!(millis_until_next_second(1_500), 500);
}

#[test]
fn millis_until_next_second_just_before_boundary() {
    assert_eq!(millis_until_next_second(1_999), 1);
}

// --- settings defaults ---

#[test]
fn defaults_match_the_contract() {
    let settings = Settings::default();
    assert_eq!(settings.version, 1);
    assert!(settings.format_24h);
    assert!(!settings.show_seconds);
    assert!(!settings.show_date);
    assert!(!settings.pinned);
    assert_eq!(settings.corner, Corner::TopRight);
    assert_eq!(settings.scale, Scale::Medium);
}

// --- settings stored-file parsing (lenient) ---

#[test]
fn stored_settings_empty_string_falls_back_to_defaults() {
    assert_eq!(settings_from_json(""), Settings::default());
}

#[test]
fn stored_settings_invalid_json_falls_back_to_defaults() {
    assert_eq!(settings_from_json("{not json"), Settings::default());
}

#[test]
fn stored_settings_non_object_falls_back_to_defaults() {
    assert_eq!(settings_from_json("[1, 2]"), Settings::default());
}

#[test]
fn stored_settings_missing_fields_default() {
    let settings = settings_from_json(r#"{"show_seconds": true}"#);
    assert!(settings.show_seconds);
    assert_eq!(settings.version, 1);
    assert!(settings.format_24h);
    assert_eq!(settings.corner, Corner::TopRight);
    assert_eq!(settings.scale, Scale::Medium);
}

#[test]
fn stored_settings_unknown_fields_are_ignored() {
    let settings = settings_from_json(r#"{"scale": "large", "theme": "neon", "x": [1]}"#);
    assert_eq!(settings.scale, Scale::Large);
    assert_eq!(settings.corner, Corner::TopRight);
}

#[test]
fn stored_settings_invalid_field_falls_back_alone() {
    let settings = settings_from_json(r#"{"format_24h": "yes", "show_date": true, "version": "two"}"#);
    assert!(settings.format_24h);
    assert!(settings.show_date);
    assert_eq!(settings.version, 1);
}

#[test]
fn stored_settings_invalid_corner_falls_back_alone() {
    let settings = settings_from_json(r#"{"corner": "middle", "scale": "small"}"#);
    assert_eq!(settings.corner, Corner::TopRight);
    assert_eq!(settings.scale, Scale::Small);
}

#[test]
fn stored_settings_invalid_scale_falls_back_alone() {
    let settings = settings_from_json(r#"{"corner": "bottom_left", "scale": 42}"#);
    assert_eq!(settings.corner, Corner::BottomLeft);
    assert_eq!(settings.scale, Scale::Medium);
}

#[test]
fn stored_settings_round_trip() {
    let settings = Settings {
        version: 1,
        format_24h: false,
        show_seconds: true,
        show_date: true,
        pinned: true,
        corner: Corner::BottomRight,
        scale: Scale::Large,
    };
    let json = serde_json::to_string(&settings).expect("serialize");
    assert_eq!(settings_from_json(&json), settings);
}

#[test]
fn stored_settings_use_snake_case_enum_values() {
    let json = serde_json::to_string(&Settings::default()).expect("serialize");
    assert!(json.contains("\"top_right\""), "json was {json}");
    assert!(json.contains("\"medium\""), "json was {json}");
}

// --- settings IPC payload (strict) ---

#[test]
fn payload_accepts_full_valid_object() {
    let json = r#"{
        "version": 2,
        "format_24h": false,
        "show_seconds": true,
        "show_date": true,
        "pinned": true,
        "corner": "bottom_left",
        "scale": "large"
    }"#;
    let settings = validate_settings_payload(json).expect("valid");
    assert_eq!(
        settings,
        Settings {
            version: 2,
            format_24h: false,
            show_seconds: true,
            show_date: true,
            pinned: true,
            corner: Corner::BottomLeft,
            scale: Scale::Large,
        }
    );
}

#[test]
fn payload_missing_fields_default() {
    let settings = validate_settings_payload(r#"{"show_date": true}"#).expect("valid");
    assert!(settings.show_date);
    assert_eq!(settings, Settings { show_date: true, ..Settings::default() });
}

#[test]
fn payload_ignores_unknown_fields() {
    let settings = validate_settings_payload(r#"{"scale": "small", "colour": "red"}"#).expect("valid");
    assert_eq!(settings.scale, Scale::Small);
}

#[test]
fn payload_rejects_non_object() {
    assert!(validate_settings_payload("[]").is_err());
    assert!(validate_settings_payload("null").is_err());
    assert!(validate_settings_payload("7").is_err());
    assert!(validate_settings_payload("\"clock\"").is_err());
    assert!(validate_settings_payload("nonsense").is_err());
}

#[test]
fn payload_rejects_wrong_field_type() {
    assert!(validate_settings_payload(r#"{"format_24h": "yes"}"#).is_err());
    assert!(validate_settings_payload(r#"{"show_seconds": 1}"#).is_err());
    assert!(validate_settings_payload(r#"{"pinned": null}"#).is_err());
}

#[test]
fn payload_rejects_invalid_corner() {
    assert!(validate_settings_payload(r#"{"corner": "middle"}"#).is_err());
    assert!(validate_settings_payload(r#"{"corner": 3}"#).is_err());
}

#[test]
fn payload_rejects_invalid_scale() {
    assert!(validate_settings_payload(r#"{"scale": "huge"}"#).is_err());
    assert!(validate_settings_payload(r#"{"scale": true}"#).is_err());
}

#[test]
fn payload_rejects_invalid_version() {
    assert!(validate_settings_payload(r#"{"version": 0}"#).is_err());
    assert!(validate_settings_payload(r#"{"version": -1}"#).is_err());
    assert!(validate_settings_payload(r#"{"version": 1.5}"#).is_err());
    assert!(validate_settings_payload(r#"{"version": "1"}"#).is_err());
}

#[test]
fn payload_error_mentions_the_offending_field() {
    let error = validate_settings_payload(r#"{"corner": "middle"}"#).expect_err("invalid");
    assert!(error.to_string().contains("corner"), "error was {error}");
    assert!(!error.to_string().is_empty());
}

// --- settings persistence ---

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn load_missing_file_returns_defaults() {
    let dir = temp_dir();
    assert_eq!(load_settings(&dir.path().join("settings.json")), Settings::default());
}

#[test]
fn load_corrupt_file_returns_defaults() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    fs::write(&path, "{oops").expect("write");
    assert_eq!(load_settings(&path), Settings::default());
}

#[test]
fn save_creates_parent_directories() {
    let dir = temp_dir();
    let path = dir.path().join("nested/deeper/settings.json");
    save_settings(&path, &Settings::default()).expect("save");
    assert!(path.is_file());
}

#[test]
fn save_then_load_keeps_values() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    let settings = Settings { show_seconds: true, scale: Scale::Small, ..Settings::default() };
    save_settings(&path, &settings).expect("save");
    assert_eq!(load_settings(&path), settings);
}

#[test]
fn saving_twice_replaces_previous_file() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    let first = Settings { show_seconds: true, ..Settings::default() };
    let second = Settings { show_date: true, corner: Corner::BottomLeft, ..Settings::default() };
    save_settings(&path, &first).expect("first save");
    save_settings(&path, &second).expect("second save");
    assert_eq!(load_settings(&path), second);
}

#[test]
fn save_leaves_no_temporary_files() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    save_settings(&path, &Settings::default()).expect("save");

    let mut names: Vec<String> = fs::read_dir(dir.path())
        .expect("read_dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["settings.json".to_string()]);
}

#[test]
fn failed_save_leaves_previous_file_intact() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    let original = Settings { show_seconds: true, ..Settings::default() };
    save_settings(&path, &original).expect("save");
    let before = fs::read_to_string(&path).expect("read");

    let read_only = dir.path().join("locked");
    fs::create_dir(&read_only).expect("mkdir");
    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o500)).expect("chmod");
    let locked_path = read_only.join("settings.json");
    let result = save_settings(&locked_path, &Settings::default());
    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o700)).expect("chmod back");

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).expect("read"), before);
}

#[test]
fn save_over_a_corrupt_file_succeeds() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    fs::write(&path, "{oops").expect("write");
    let settings = Settings { pinned: true, ..Settings::default() };
    save_settings(&path, &settings).expect("save");
    assert_eq!(load_settings(&path), settings);
}

#[test]
fn load_tolerates_a_directory_in_place_of_the_file() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    fs::create_dir(&path).expect("mkdir");
    assert_eq!(load_settings(&path), Settings::default());
}

#[test]
fn save_to_a_path_that_is_a_directory_fails() {
    let dir = temp_dir();
    let path = dir.path().join("settings.json");
    fs::create_dir(&path).expect("mkdir");
    assert!(save_settings(&path, &Settings::default()).is_err());
}

// --- window mode ---

#[test]
fn pinned_with_layer_shell_is_pinned() {
    assert_eq!(resolve_window_mode(true, true), WindowMode::Pinned);
}

#[test]
fn pinned_without_layer_shell_is_windowed() {
    assert_eq!(resolve_window_mode(true, false), WindowMode::Windowed);
}

#[test]
fn unpinned_with_layer_shell_is_windowed() {
    assert_eq!(resolve_window_mode(false, true), WindowMode::Windowed);
}

#[test]
fn unpinned_without_layer_shell_is_windowed() {
    assert_eq!(resolve_window_mode(false, false), WindowMode::Windowed);
}

// --- geometry ---

fn size(width: f64, height: f64) -> Size {
    Size { width, height }
}

#[test]
fn medium_hour_minute_size() {
    assert_eq!(window_size(Scale::Medium, false, false), size(464.0, 168.0));
}

#[test]
fn small_hour_minute_size() {
    assert_eq!(window_size(Scale::Small, false, false), size(336.0, 124.0));
}

#[test]
fn large_hour_minute_size() {
    assert_eq!(window_size(Scale::Large, false, false), size(656.0, 236.0));
}

#[test]
fn medium_with_seconds_and_date_size() {
    assert_eq!(window_size(Scale::Medium, true, true), size(632.0, 204.0));
}

#[test]
fn medium_with_seconds_only_size() {
    assert_eq!(window_size(Scale::Medium, true, false), size(632.0, 168.0));
}

#[test]
fn medium_with_date_only_size() {
    assert_eq!(window_size(Scale::Medium, false, true), size(464.0, 204.0));
}

#[test]
fn small_with_seconds_and_date_size() {
    assert_eq!(window_size(Scale::Small, true, true), size(464.0, 160.0));
}

#[test]
fn large_with_seconds_and_date_size() {
    assert_eq!(window_size(Scale::Large, true, true), size(888.0, 272.0));
}
