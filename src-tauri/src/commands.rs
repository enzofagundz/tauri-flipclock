//! IPC commands exposed to the webview.

use flipclock_core::mode::WindowMode;
use flipclock_core::settings::Corner;
use tauri::{AppHandle, LogicalSize, Runtime, State, WebviewWindow};

use crate::layer_shell;
use crate::state::{AppState, Bootstrap, SaveResult};

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Result<Bootstrap, String> {
    Ok(state.bootstrap(chrono::Local::now()))
}

#[tauri::command]
pub fn save_settings<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<SaveResult, String> {
    let previous = state.settings();
    let result = state.save(&settings)?;

    let display_changed = previous.scale != result.settings.scale
        || previous.show_seconds != result.settings.show_seconds
        || previous.show_date != result.settings.show_date;
    if display_changed {
        resize(&window, &state);
    }

    if previous.corner != result.settings.corner && state.window_mode() == WindowMode::Pinned {
        apply_anchors(&window, result.settings.corner, state.is_fullscreen());
    }

    Ok(result)
}

#[tauri::command]
pub fn set_fullscreen<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    commit_fullscreen(&state, enabled, || {
        if state.window_mode() == WindowMode::Pinned {
            apply_anchors(&window, state.settings().corner, enabled);
            if !enabled {
                resize(&window, &state);
            }
            Ok(())
        } else {
            window
                .set_fullscreen(enabled)
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
fn apply_anchors<R: Runtime>(window: &WebviewWindow<R>, corner: Corner, fullscreen: bool) {
    let window = window.clone();
    let scheduled = window.clone().run_on_main_thread(move || {
        if let Some(gtk_window) = gtk_window(&window) {
            layer_shell::apply_anchors(&gtk_window, corner, fullscreen);
        }
    });
    if let Err(error) = scheduled {
        eprintln!("flipclock: cannot schedule the layer-shell update: {error}");
    }
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
                set_fullscreen
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
}
