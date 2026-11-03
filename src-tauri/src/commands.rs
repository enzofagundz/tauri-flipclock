//! IPC commands exposed to the webview.

use flipclock_core::mode::WindowMode;
use flipclock_core::now_playing::is_browser_player;
use flipclock_core::settings::Corner;
use serde::Deserialize;
use tauri::{AppHandle, LogicalSize, Runtime, State, WebviewWindow};

use crate::layer_shell;
use crate::media;
use crate::state::{AppState, Bootstrap, SaveResult};

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Result<Bootstrap, String> {
    Ok(state.bootstrap(chrono::Local::now()))
}

#[tauri::command]
pub fn save_settings<R: Runtime>(
    app: AppHandle<R>,
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<SaveResult, String> {
    let previous = state.settings();
    let result = state.save(&settings)?;
    state.sync_media_worker(&app, result.settings.show_now_playing);

    let display_changed = previous.scale != result.settings.scale
        || previous.show_seconds != result.settings.show_seconds
        || previous.show_date != result.settings.show_date;
    let corner_changed = previous.corner != result.settings.corner;

    if state.window_mode() == WindowMode::Pinned {
        if display_changed || corner_changed {
            apply_geometry(
                &window,
                result.settings.corner,
                state.is_fullscreen(),
                pinned_size(&state, state.is_fullscreen()),
            );
        }
    } else if display_changed && !state.is_fullscreen() {
        resize(&window, &state);
    }

    Ok(result)
}

/// Returns the cached artwork bytes for the selected player.
///
/// The allowlist is checked first and the cached path is re-validated, so a
/// stale or hostile path cannot be used to read anything outside an artwork
/// file the reader already accepted.
#[tauri::command]
pub fn get_artwork(
    state: State<'_, AppState>,
    player: String,
) -> Result<tauri::ipc::Response, String> {
    if !is_browser_player(&player) {
        return Err(format!("`{player}` is not an allowlisted browser"));
    }

    let resource = state
        .artwork()
        .filter(|resource| resource.player == player)
        .ok_or_else(|| format!("no cached artwork for `{player}`"))?;

    let (mime, bytes) = media::read_artwork(&resource.path)?;
    if mime != resource.mime {
        return Err("cached artwork type changed on disk".to_string());
    }

    Ok(tauri::ipc::Response::new(bytes))
}

/// Transport actions the now-playing card can send to the player.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCommand {
    PlayPause,
    Previous,
    Next,
}

impl MediaCommand {
    fn member(self) -> &'static str {
        match self {
            MediaCommand::PlayPause => "PlayPause",
            MediaCommand::Previous => "Previous",
            MediaCommand::Next => "Next",
        }
    }
}

#[tauri::command]
pub fn media_command(player: String, command: MediaCommand) -> Result<(), String> {
    media::send_command(&player, command.member())
}

#[tauri::command]
pub fn set_fullscreen<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    commit_fullscreen(&state, enabled, || {
        if state.window_mode() == WindowMode::Pinned {
            apply_geometry(
                &window,
                state.settings().corner,
                enabled,
                pinned_size(&state, enabled),
            );
            Ok(())
        } else if enabled {
            // A non-resizable GTK window ignores the fullscreen request.
            window
                .set_resizable(true)
                .map_err(|error| error.to_string())?;
            window
                .set_fullscreen(true)
                .map_err(|error| error.to_string())?;
            // Safety net for a compositor that keeps the old surface size.
            resize_to_monitor(&window);
            Ok(())
        } else {
            window
                .set_fullscreen(false)
                .map_err(|error| error.to_string())?;
            resize(&window, &state);
            window
                .set_resizable(false)
                .map_err(|error| error.to_string())
        }
    })
}

/// Runs the windowing change first and only stores the state once it succeeded.
///
/// Keeping the store after the operation means a failed window call cannot leave
/// `is_fullscreen` disagreeing with the actual window.
fn commit_fullscreen(
    state: &AppState,
    enabled: bool,
    operation: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    if enabled == state.is_fullscreen() {
        return Ok(enabled);
    }
    operation()?;
    Ok(state.set_fullscreen(enabled))
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

fn resize<R: Runtime>(window: &WebviewWindow<R>, state: &AppState) {
    let size = state.window_size();
    if let Err(error) = window.set_size(LogicalSize::new(size.width, size.height)) {
        eprintln!("flipclock: cannot resize the window: {error}");
    }
}

/// Sizes the window to the monitor it sits on.
///
/// Used after a fullscreen request as a fallback: some compositors keep the old
/// surface size when the window itself refuses to become fullscreen.
fn resize_to_monitor<R: Runtime>(window: &WebviewWindow<R>) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| {
            window
                .available_monitors()
                .ok()
                .and_then(|monitors| monitors.into_iter().next())
        });
    let Some(monitor) = monitor else {
        eprintln!("flipclock: no monitor is available for fullscreen");
        return;
    };

    let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
    if let Err(error) = window.set_size(LogicalSize::new(size.width, size.height)) {
        eprintln!("flipclock: cannot size the fullscreen window: {error}");
    }
}

fn gtk_window<R: Runtime>(window: &WebviewWindow<R>) -> Option<gtk::ApplicationWindow> {
    match window.gtk_window() {
        Ok(gtk_window) => Some(gtk_window),
        Err(error) => {
            eprintln!("flipclock: cannot reach the GTK window: {error}");
            None
        }
    }
}

/// Layer-shell calls must run on the GTK main thread; a command may not.
fn apply_geometry<R: Runtime>(
    window: &WebviewWindow<R>,
    corner: Corner,
    fullscreen: bool,
    size: Option<(i32, i32)>,
) {
    let window = window.clone();
    let scheduled = window.clone().run_on_main_thread(move || {
        if let Some(gtk_window) = gtk_window(&window) {
            layer_shell::apply_geometry(&gtk_window, corner, fullscreen, size);
        }
    });
    if let Err(error) = scheduled {
        eprintln!("flipclock: cannot schedule the layer-shell update: {error}");
    }
}

/// The compositor owns the size in fullscreen; otherwise it is the clock size.
fn pinned_size(state: &AppState, fullscreen: bool) -> Option<(i32, i32)> {
    if fullscreen {
        return None;
    }
    let size = state.window_size();
    Some((size.width as i32, size.height as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SETTINGS_FILE;
    use serde_json::{json, Value};
    use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
    use tauri::test::{get_ipc_response, mock_builder, MockRuntime};
    use tauri::webview::InvokeRequest;
    use tauri::{App, Manager, WebviewUrl, WebviewWindowBuilder};
    use tempfile::TempDir;

    fn mock_app() -> (TempDir, App<MockRuntime>) {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join(SETTINGS_FILE);
        let app = mock_builder()
            .invoke_handler(tauri::generate_handler![
                get_bootstrap,
                save_settings,
                set_fullscreen,
                get_artwork,
                media_command
            ])
            .build(tauri::generate_context!())
            .unwrap();
        // The real app manages the state in its setup hook, which only runs with
        // the event loop; the command tests manage it directly.
        app.manage(AppState::new(settings_path, true));
        WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
            .build()
            .unwrap();
        (dir, app)
    }

    fn invoke(app: &App<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
        let webview = app.get_webview_window("main").unwrap();
        let request = InvokeRequest {
            cmd: cmd.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        get_ipc_response(&webview, request).map(|response| match response {
            InvokeResponseBody::Json(json) => serde_json::from_str(&json).unwrap(),
            InvokeResponseBody::Raw(bytes) => serde_json::from_slice(&bytes).unwrap(),
        })
    }

    #[test]
    fn failed_fullscreen_operation_leaves_the_state_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join(SETTINGS_FILE), true);

        let error =
            commit_fullscreen(&state, true, || Err("no fullscreen here".to_string())).unwrap_err();

        assert_eq!(error, "no fullscreen here");
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn successful_fullscreen_operation_stores_the_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join(SETTINGS_FILE), true);

        assert!(commit_fullscreen(&state, true, || Ok(())).unwrap());
        assert!(state.is_fullscreen());
        assert!(!commit_fullscreen(&state, false, || Ok(())).unwrap());
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn fullscreen_no_op_does_not_run_the_window_operation() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join(SETTINGS_FILE), true);

        let result = commit_fullscreen(&state, false, || panic!("must not run"));

        assert!(!result.unwrap());
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn get_bootstrap_returns_the_frontend_contract() {
        let (_dir, app) = mock_app();
        let value = invoke(&app, "get_bootstrap", json!({})).unwrap();

        assert!(value["settings"].is_object());
        assert!(value["reading"]["hour"].is_string());
        assert_eq!(value["layer_shell_available"], json!(true));
        assert_eq!(value["window_mode"], json!("windowed"));
        assert_eq!(value["now_playing"], json!(null));
    }

    #[test]
    fn save_settings_accepts_the_settings_argument() {
        let (_dir, app) = mock_app();
        let settings = json!({
            "version": 1,
            "format_24h": false,
            "show_seconds": true,
            "show_date": true,
            "pinned": false,
            "show_now_playing": false,
            "corner": "bottom_left",
            "scale": "large",
        });

        let value = invoke(&app, "save_settings", json!({ "settings": settings })).unwrap();

        assert_eq!(value["restart_required"], json!(false));
        assert_eq!(value["settings"]["corner"], json!("bottom_left"));
        assert_eq!(value["settings"]["show_seconds"], json!(true));
    }

    #[test]
    fn save_settings_rejects_an_invalid_payload() {
        let (_dir, app) = mock_app();
        let error = invoke(
            &app,
            "save_settings",
            json!({ "settings": { "scale": "gigantic" } }),
        )
        .unwrap_err();

        assert!(
            error.as_str().unwrap_or_default().contains("scale"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn set_fullscreen_reports_the_state() {
        let (_dir, app) = mock_app();

        let value = invoke(&app, "set_fullscreen", json!({ "enabled": true })).unwrap();
        assert_eq!(value, json!(true));

        let value = invoke(&app, "set_fullscreen", json!({ "enabled": false })).unwrap();
        assert_eq!(value, json!(false));
    }

    #[test]
    fn get_artwork_rejects_a_player_outside_the_allowlist() {
        let (_dir, app) = mock_app();
        let error = invoke(
            &app,
            "get_artwork",
            json!({ "player": "org.mpris.MediaPlayer2.spotify" }),
        )
        .unwrap_err();

        assert!(
            error.as_str().unwrap_or_default().contains("allowlisted"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn get_artwork_rejects_an_allowlisted_player_without_cached_artwork() {
        let (_dir, app) = mock_app();
        let error = invoke(
            &app,
            "get_artwork",
            json!({ "player": "org.mpris.MediaPlayer2.firefox.instance1" }),
        )
        .unwrap_err();

        assert!(
            error
                .as_str()
                .unwrap_or_default()
                .contains("no cached artwork"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn media_command_rejects_a_player_outside_the_allowlist() {
        let (_dir, app) = mock_app();
        let error = invoke(
            &app,
            "media_command",
            json!({ "player": "org.mpris.MediaPlayer2.spotify", "command": "play_pause" }),
        )
        .unwrap_err();

        assert!(
            error.as_str().unwrap_or_default().contains("allowlisted"),
            "unexpected error: {error}"
        );
    }
}
