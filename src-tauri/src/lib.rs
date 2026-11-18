pub mod models;
pub mod runner;
pub mod system;
pub mod tasks;
pub mod updater;

use models::{SystemInfo, TaskDef, TaskResult, UpdateInfo};
use runner::{emit_log, RunError, RunnerState};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
async fn get_system_info() -> Result<SystemInfo, String> {
    tokio::task::spawn_blocking(system::collect_system_info)
        .await
        .map_err(|e| format!("Failed to collect system info: {e}"))
}

#[tauri::command]
async fn get_tasks() -> Result<Vec<TaskDef>, String> {
    let info = get_system_info().await?;
    Ok(system::build_task_list(&info))
}

#[tauri::command]
async fn run_task(
    app: AppHandle,
    state: State<'_, RunnerState>,
    task_id: String,
    options: HashMap<String, bool>,
) -> Result<TaskResult, String> {
    let _guard = state.begin_run()?;
    emit_log(&app, &state, &task_id, &format!("Starting task '{task_id}'..."), "info");

    match tasks::dispatch_task(&app, &state, &task_id, &options).await {
        Ok(result) => {
            emit_log(&app, &state, &task_id, &result.summary, "info");
            Ok(result)
        }
        Err(RunError::Canceled) => {
            emit_log(&app, &state, &task_id, "Task canceled by user.", "info");
            Ok(tasks::canceled_result(&task_id))
        }
        Err(RunError::Message(msg)) => {
            emit_log(&app, &state, &task_id, &msg, "stderr");
            Err(msg)
        }
    }
}

#[tauri::command]
fn cancel_task(state: State<'_, RunnerState>) -> Result<(), String> {
    state.cancel_active();
    Ok(())
}

#[tauri::command]
async fn check_updates() -> Option<UpdateInfo> {
    updater::check_latest_release(env!("CARGO_PKG_VERSION")).await
}

#[tauri::command]
fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("mailto:")) {
        return Err("Only https:// and mailto: URLs are allowed.".to_string());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {e}"))
}

#[tauri::command]
fn open_software_update_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.Software-Update-Settings.extension",
                None::<&str>,
            )
            .map_err(|e| format!("Failed to open System Settings: {e}"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Software Update Settings shortcut is only available on macOS.".to_string())
    }
}

#[tauri::command]
fn relaunch_as_admin(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        tasks::windows::relaunch_as_admin()?;
        app.exit(0);
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("Relaunch as administrator is only supported on Windows.".to_string())
    }
}

#[tauri::command]
fn report_error(app: AppHandle, state: State<'_, RunnerState>) -> Result<String, String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create log directory: {e}"))?;

    let log_path = log_dir.join("maintenance-error.log");
    let info = system::collect_system_info();
    let epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let contents = format!(
        "=== System Maintenance Toolkit Error Log ===\n\
         App Version: {}\n\
         OS: {} ({})\n\
         Admin: {}\n\
         Homebrew: {}\n\
         Unix Timestamp: {}\n\n\
         {}",
        info.app_version,
        info.os_version,
        info.arch,
        info.is_admin,
        info.brew_version.unwrap_or_else(|| "Not installed".to_string()),
        epoch_secs,
        state.get_log()
    );

    std::fs::write(&log_path, contents)
        .map_err(|e| format!("Failed to write error log: {e}"))?;

    let _ = app.opener().reveal_item_in_dir(&log_path);
    let _ = app.opener().open_url(
        "mailto:lzdrahal@gmail.com?subject=System%20Maintenance%20Toolkit%20Error%20Report&body=An%20issue%20occurred.%20I%20have%20attached%20maintenance-error.log%20from%20the%20folder%20that%20just%20opened.",
        None::<&str>,
    );

    Ok(log_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_session_log(state: State<'_, RunnerState>) -> String {
    state.get_log()
}

#[tauri::command]
fn clear_session_log(state: State<'_, RunnerState>) {
    state.clear_log();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(RunnerState::default())
        .on_window_event(|window, event| {
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) {
                window.state::<RunnerState>().cancel_active();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_tasks,
            run_task,
            cancel_task,
            check_updates,
            open_external_url,
            open_software_update_settings,
            relaunch_as_admin,
            report_error,
            get_session_log,
            clear_session_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
