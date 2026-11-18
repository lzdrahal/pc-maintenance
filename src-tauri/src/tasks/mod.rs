pub mod macos;
pub mod windows;

use crate::models::{TaskResult, TaskStatus};
use crate::runner::{RunError, RunnerState};
use std::collections::HashMap;
use tauri::AppHandle;

pub async fn dispatch_task(
    app: &AppHandle,
    state: &RunnerState,
    task_id: &str,
    options: &HashMap<String, bool>,
) -> Result<TaskResult, RunError> {
    #[cfg(target_os = "macos")]
    {
        return macos::run_macos_task(app, state, task_id, options).await;
    }

    #[cfg(target_os = "windows")]
    {
        return windows::run_windows_task(app, state, task_id, options).await;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (app, state, options);
        Ok(TaskResult {
            task_id: task_id.to_string(),
            status: TaskStatus::Failed,
            summary: format!("Unsupported operating system: {}", std::env::consts::OS),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        })
    }
}

pub fn canceled_result(task_id: &str) -> TaskResult {
    TaskResult {
        task_id: task_id.to_string(),
        status: TaskStatus::Canceled,
        summary: "Canceled by user.".to_string(),
        bytes_freed: None,
        reboot_required: false,
        open_settings_hint: false,
    }
}
