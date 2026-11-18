use crate::models::{TaskResult, TaskStatus};
#[cfg(target_os = "windows")]
use crate::runner::{emit_log, format_bytes, stream_command, RunError, RunnerState};
#[cfg(target_os = "windows")]
use crate::system::get_disk_space;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use tauri::{AppHandle, Manager};
#[cfg(target_os = "windows")]
use tokio::process::Command;

pub const WINGET_NO_UPDATE_CODE: i32 = -1978335189; // 0x8A15002B

pub fn classify_winget_result(exit_code: i32, combined_output: &str) -> TaskResult {
    let lower = combined_output.to_lowercase();
    if exit_code == 0 {
        let up_to_date = lower.contains("all winget packages are up to date")
            || lower.contains("no applicable update found");
        return TaskResult {
            task_id: "win-winget".to_string(),
            status: TaskStatus::Success,
            summary: if up_to_date {
                "All Winget packages are up to date.".to_string()
            } else {
                "Winget software updates completed.".to_string()
            },
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        };
    }

    if exit_code == WINGET_NO_UPDATE_CODE || lower.contains("no applicable update found") {
        return TaskResult {
            task_id: "win-winget".to_string(),
            status: TaskStatus::Success,
            summary: "All Winget packages are up to date.".to_string(),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        };
    }

    TaskResult {
        task_id: "win-winget".to_string(),
        status: TaskStatus::Failed,
        summary: format!("Winget exited with code {exit_code}. Check Technical Output."),
        bytes_freed: None,
        reboot_required: false,
        open_settings_hint: false,
    }
}

#[cfg(target_os = "windows")]
fn resolve_script_path(app: &AppHandle, script_name: &str) -> Result<PathBuf, RunError> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("scripts").join(script_name);
        if bundled.is_file() {
            return Ok(bundled);
        }
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../src")
        .join(script_name);
    if dev_path.is_file() {
        return Ok(dev_path);
    }

    Err(RunError::Message(format!(
        "Could not locate maintenance script: {script_name}"
    )))
}

#[cfg(target_os = "windows")]
pub async fn run_windows_task(
    app: &AppHandle,
    state: &RunnerState,
    task_id: &str,
    options: &HashMap<String, bool>,
) -> Result<TaskResult, RunError> {
    let (script_name, extra_args): (&str, Vec<&str>) = match task_id {
        "win-update" => ("Update-Windows.ps1", vec![]),
        "win-winget" => ("Update-Software.ps1", vec![]),
        "win-cleanup" => {
            let empty_recycle = options.get("empty_recycle_bin").copied().unwrap_or(false);
            if empty_recycle {
                ("Cleanup-System.ps1", vec![])
            } else {
                ("Cleanup-System.ps1", vec!["-SkipRecycleBin"])
            }
        }
        "win-repair" => ("Repair-System.ps1", vec![]),
        "win-network" => ("Optimize-Network.ps1", vec![]),
        _ => return Err(RunError::Message(format!("Unknown Windows task: {task_id}"))),
    };

    let script_path = resolve_script_path(app, script_name)?;
    emit_log(
        app,
        state,
        task_id,
        &format!("Running {}...", script_path.display()),
        "info",
    );

    let (free_before, _) = if task_id == "win-cleanup" {
        get_disk_space()
    } else {
        (0, 0)
    };

    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .args(&extra_args);

    let outcome = stream_command(app, state, task_id, cmd).await?;
    let combined = format!("{}\n{}", outcome.stdout, outcome.stderr);
    let lower = combined.to_lowercase();

    if task_id == "win-winget" {
        return Ok(classify_winget_result(outcome.exit_code, &combined));
    }

    let reboot_required = combined.contains("REBOOT_REQUIRED:")
        || lower.contains("reboot is required")
        || lower.contains("restart may be required");

    if outcome.exit_code != 0 {
        return Ok(TaskResult {
            task_id: task_id.to_string(),
            status: TaskStatus::Failed,
            summary: format!(
                "{script_name} exited with code {}. Check Technical Output.",
                outcome.exit_code
            ),
            bytes_freed: None,
            reboot_required,
            open_settings_hint: false,
        });
    }

    if task_id == "win-cleanup" {
        let (free_after, _) = get_disk_space();
        let freed = free_after.saturating_sub(free_before);
        let summary = if freed > 0 {
            format!("System cleanup completed · Freed {}", format_bytes(freed))
        } else {
            "System cleanup completed.".to_string()
        };
        return Ok(TaskResult {
            task_id: task_id.to_string(),
            status: TaskStatus::Success,
            summary,
            bytes_freed: (freed > 0).then_some(freed),
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    let summary = match task_id {
        "win-update" => {
            if lower.contains("no new windows updates found") {
                "Windows is up to date.".to_string()
            } else if reboot_required {
                "Windows updates installed. Restart required.".to_string()
            } else {
                "Windows updates completed.".to_string()
            }
        }
        "win-repair" => "DISM and SFC system integrity checks passed.".to_string(),
        "win-network" => {
            if reboot_required {
                "Network reset completed. Restart may be required.".to_string()
            } else {
                "DNS cache flushed and network stack reset.".to_string()
            }
        }
        _ => "Task completed successfully.".to_string(),
    };

    Ok(TaskResult {
        task_id: task_id.to_string(),
        status: TaskStatus::Success,
        summary,
        bytes_freed: None,
        reboot_required,
        open_settings_hint: false,
    })
}

#[cfg(target_os = "windows")]
pub fn relaunch_as_admin() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Could not resolve current executable path: {e}"))?;
    let exe_escaped = exe_path.to_string_lossy().replace('\'', "''");
    let ps_cmd = format!("Start-Process -FilePath '{exe_escaped}' -Verb RunAs");

    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to request administrator elevation: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_winget_no_update_code_as_success() {
        let res = classify_winget_result(WINGET_NO_UPDATE_CODE, "");
        assert_eq!(res.status, TaskStatus::Success);
        assert!(res.summary.contains("up to date"));

        let res_err = classify_winget_result(1, "network error");
        assert_eq!(res_err.status, TaskStatus::Failed);
    }
}
