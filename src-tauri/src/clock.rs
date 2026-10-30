//! The one-second tick loop that feeds the frontend.

use std::thread;
use std::time::Duration;

use chrono::Local;
use flipclock_core::reading::format_reading;
use flipclock_core::tick::{classify_tick, millis_until_next_second, TickKind};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub const TICK_EVENT: &str = "clock:tick";

/// Starts the tick thread. It ends with the process.
pub fn spawn(app: AppHandle) {
    let result = thread::Builder::new()
        .name("flipclock-tick".to_string())
        .spawn(move || run(app));
    if let Err(error) = result {
        eprintln!("flipclock: cannot start the tick thread: {error}");
    }
}

fn run(app: AppHandle) {
    let mut previous = None;
    loop {
        let now = Local::now();
        let now_millis = now.timestamp_millis();
        let animate = classify_tick(previous, now_millis) == TickKind::Normal;
        previous = Some(now_millis);

        let reading = {
            let state = app.state::<AppState>();
            format_reading(now, &state.settings(), animate)
        };

        if let Err(error) = app.emit(TICK_EVENT, reading) {
            eprintln!("flipclock: cannot emit {TICK_EVENT}: {error}");
        }

        thread::sleep(Duration::from_millis(millis_until_next_second(now_millis)));
    }
}
