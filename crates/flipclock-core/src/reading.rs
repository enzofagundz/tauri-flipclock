use chrono::{DateTime, Datelike, TimeZone, Timelike};
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reading {
    pub hour: String,
    pub minute: String,
    pub second: String,
    pub date: String,
    pub meridiem: Option<String>,
    pub animate: bool,
}

/// Builds the display reading for `now`, honouring the 12/24-hour setting.
///
/// `meridiem` is `Some("AM")`/`Some("PM")` only in 12-hour mode; the date is
/// always `DD/MM/YYYY`.
pub fn format_reading<Tz: TimeZone>(now: DateTime<Tz>, settings: &Settings, animate: bool) -> Reading {
    let (hour, meridiem) = if settings.format_24h {
        (format!("{:02}", now.hour()), None)
    } else {
        let hour12 = match now.hour() % 12 {
            0 => 12,
            hour => hour,
        };
        let meridiem = if now.hour() < 12 { "AM" } else { "PM" };
        (format!("{hour12:02}"), Some(meridiem.to_string()))
    };

    Reading {
        hour,
        minute: format!("{:02}", now.minute()),
        second: format!("{:02}", now.second()),
        date: format!("{:02}/{:02}/{:04}", now.day(), now.month(), now.year()),
        meridiem,
        animate,
    }
}
