//! Flipclock application shell: windowing, IPC and the tick thread.

mod clock;
mod commands;
mod layer_shell;
mod media;
mod state;

use std::env;

use flipclock_core::mode::WindowMode;
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

use crate::state::{AppState, SETTINGS_FILE};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main(app);
        }))
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::save_settings,
            commands::set_fullscreen,
            commands::get_artwork,
            commands::media_command,
            commands::quit,
        ])
        .setup(|app| {
            setup(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Flipclock");
}

fn setup(app: &mut tauri::App) {
    let config_dir = app.path().app_config_dir().unwrap_or_else(|error| {
        eprintln!("flipclock: cannot resolve the config directory: {error}; using a temp dir");
        env::temp_dir().join("flipclock")
    });

    let layer_shell_available = layer_shell::is_supported() && !layer_shell_disabled();
    let state = AppState::new(config_dir.join(SETTINGS_FILE), layer_shell_available);

    let Some(window) = app.get_webview_window("main") else {
        eprintln!("flipclock: the `main` window is missing");
        app.manage(state);
        return;
    };

    app.manage(state);
    let handle = app.handle().clone();

    {
        let state = handle.state::<AppState>();
        apply_startup_mode(&window, &state);

        // Populate the model and start the reader before the first frame, so the
        // frontend never renders an empty card while the bus is read.
        if state.settings().show_now_playing {
            media::snapshot_once(&handle);
            state.sync_media_worker(&handle, true);
        }
    }

    if let Err(error) = window.show() {
        eprintln!("flipclock: cannot show the window: {error}");
    }
    if let Err(error) = window.set_always_on_top(true) {
        eprintln!("flipclock: always-on-top is unavailable: {error}");
    }

    clock::spawn(handle);
}

/// Sizes the window and turns it into a layer surface before it is shown.
fn apply_startup_mode(window: &WebviewWindow, state: &AppState) {
    let size = state.window_size();

    if state.window_mode() != WindowMode::Pinned {
        if let Err(error) = window.set_size(LogicalSize::new(size.width, size.height)) {
            eprintln!("flipclock: cannot size the window: {error}");
        }
        return;
    }

    let pinned = match window.gtk_window() {
        Ok(gtk_window) => {
            let ready = layer_shell::setup(&gtk_window);
            if ready {
                layer_shell::apply_geometry(
                    &gtk_window,
                    state.settings().corner,
                    false,
                    Some((size.width as i32, size.height as i32)),
                );
            }
            ready
        }
        Err(error) => {
            eprintln!("flipclock: cannot reach the GTK window: {error}");
            false
        }
    };

    if !pinned {
        eprintln!("flipclock: layer-shell setup failed; falling back to windowed mode");
        state.force_windowed();
    }
}

fn focus_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn layer_shell_disabled() -> bool {
    env::var("FLIPCLOCK_DISABLE_LAYER_SHELL").as_deref() == Ok("1")
}
