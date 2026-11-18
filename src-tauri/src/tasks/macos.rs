use crate::models::{TaskResult, TaskStatus};
use crate::runner::format_bytes;
#[cfg(target_os = "macos")]
use crate::runner::{emit_log, run_macos_admin_script, stream_command, RunError, RunnerState};
#[cfg(target_os = "macos")]
use crate::system::{build_augmented_path, detect_brew, get_disk_space};
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tokio::process::Command;

const PROTECTED_MAC_CACHE_PREFIXES: &[&str] = &[
    "com.apple.",
    "CloudKit",
    "FamilyCircle",
    "GeoServices",
    "Metadata",
    "PassKit",
    "Spotlight",
    "VoiceTrigger",
    "Biome",
    "Siri",
    "Assistant",
    "homed",
];

pub fn is_protected_mac_cache_name(name: &str) -> bool {
    PROTECTED_MAC_CACHE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

pub fn parse_brew_upgraded_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("==> Upgrading ") && !trimmed.ends_with(':')
        })
        .count()
}

pub fn classify_brew_doctor(exit_code: i32) -> TaskStatus {
    if exit_code == 0 {
        TaskStatus::Success
    } else {
        TaskStatus::Warning
    }
}

pub fn classify_mac_update_output(exit_code: i32, combined_output: &str) -> TaskResult {
    let lower = combined_output.to_lowercase();
    let reboot_required = lower.contains("restart") || lower.contains("reboot");
    let needs_settings_auth = lower.contains("system settings")
        || lower.contains("software update preference")
        || lower.contains("requires an administrator")
        || lower.contains("owner password");

    if exit_code == 0 {
        let summary = if reboot_required {
            "macOS updates installed. Restart required to finish."
        } else {
            "macOS updates installed."
        };
        return TaskResult {
            task_id: "mac-update".to_string(),
            status: TaskStatus::Success,
            summary: summary.to_string(),
            bytes_freed: None,
            reboot_required,
            open_settings_hint: false,
        };
    }

    if needs_settings_auth {
        return TaskResult {
            task_id: "mac-update".to_string(),
            status: TaskStatus::Warning,
            summary: "macOS needs interactive confirmation in System Settings > Software Update."
                .to_string(),
            bytes_freed: None,
            reboot_required,
            open_settings_hint: true,
        };
    }

    TaskResult {
        task_id: "mac-update".to_string(),
        status: TaskStatus::Failed,
        summary: format!("softwareupdate exited with code {exit_code}. Check Technical Output."),
        bytes_freed: None,
        reboot_required,
        open_settings_hint: false,
    }
}

pub fn summarize_brew_run(upgraded_count: usize, bytes_freed: u64) -> String {
    let pkg_part = match upgraded_count {
        0 => "Homebrew packages are up to date".to_string(),
        1 => "Upgraded 1 Homebrew package".to_string(),
        n => format!("Upgraded {n} Homebrew packages"),
    };
    if bytes_freed > 0 {
        format!("{pkg_part} · Freed {}", format_bytes(bytes_freed))
    } else {
        format!("{pkg_part}.")
    }
}

#[cfg(target_os = "macos")]
pub async fn run_macos_task(
    app: &AppHandle,
    state: &RunnerState,
    task_id: &str,
    options: &HashMap<String, bool>,
) -> Result<TaskResult, RunError> {
    match task_id {
        "mac-update" => run_mac_update(app, state).await,
        "mac-brew" => run_mac_brew(app, state, options).await,
        "mac-cleanup" => run_mac_cleanup(app, state, options).await,
        "mac-health" => run_mac_health(app, state).await,
        "mac-network" => run_mac_network(app, state).await,
        _ => Err(RunError::Message(format!("Unknown macOS task: {task_id}"))),
    }
}

#[cfg(target_os = "macos")]
async fn run_mac_update(app: &AppHandle, state: &RunnerState) -> Result<TaskResult, RunError> {
    emit_log(app, state, "mac-update", "Checking for available macOS updates...", "info");

    let mut list_cmd = Command::new("/usr/sbin/softwareupdate");
    list_cmd.arg("--list");
    let list_outcome = stream_command(app, state, "mac-update", list_cmd).await?;

    let list_combined = format!("{}\n{}", list_outcome.stdout, list_outcome.stderr);
    if list_combined.to_lowercase().contains("no new software available") {
        return Ok(TaskResult {
            task_id: "mac-update".to_string(),
            status: TaskStatus::Success,
            summary: "macOS is up to date.".to_string(),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    emit_log(
        app,
        state,
        "mac-update",
        "Requesting administrator authorization to install recommended updates...",
        "info",
    );
    let outcome = run_macos_admin_script(
        app,
        state,
        "mac-update",
        "/usr/sbin/softwareupdate --install --recommended",
    )
    .await?;

    let combined = format!("{}\n{}", outcome.stdout, outcome.stderr);
    Ok(classify_mac_update_output(outcome.exit_code, &combined))
}

#[cfg(target_os = "macos")]
fn brew_command(brew_path: &Path, aug_path: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(brew_path);
    cmd.args(args)
        .env("PATH", aug_path)
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_COLOR", "0");
    cmd
}

#[cfg(target_os = "macos")]
async fn run_mac_brew(
    app: &AppHandle,
    state: &RunnerState,
    options: &HashMap<String, bool>,
) -> Result<TaskResult, RunError> {
    let brew_path = detect_brew()
        .ok_or_else(|| RunError::Message("Homebrew executable was not found.".to_string()))?;
    let aug_path = build_augmented_path(Some(&brew_path), std::env::var("PATH").ok().as_deref());

    let include_casks = options.get("include_casks").copied().unwrap_or(true);
    let cleanup_after = options.get("cleanup_after").copied().unwrap_or(true);

    let (free_before, _) = get_disk_space();
    let mut combined_upgrade_output = String::new();
    let mut warning_notes: Vec<String> = Vec::new();

    emit_log(
        app,
        state,
        "mac-brew",
        &format!("Using Homebrew at {}", brew_path.display()),
        "info",
    );

    emit_log(app, state, "mac-brew", "Step 1: Running brew update...", "info");
    let update_out = stream_command(
        app,
        state,
        "mac-brew",
        brew_command(&brew_path, &aug_path, &["update"]),
    )
    .await?;
    if update_out.exit_code != 0 {
        return Ok(TaskResult {
            task_id: "mac-brew".to_string(),
            status: TaskStatus::Failed,
            summary: format!("brew update exited with code {}.", update_out.exit_code),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    emit_log(app, state, "mac-brew", "Step 2: Upgrading Homebrew formulae...", "info");
    let formula_out = stream_command(
        app,
        state,
        "mac-brew",
        brew_command(&brew_path, &aug_path, &["upgrade", "--formula"]),
    )
    .await?;
    combined_upgrade_output.push_str(&formula_out.stdout);
    if formula_out.exit_code != 0 {
        warning_notes.push(format!("formula upgrade exited {}", formula_out.exit_code));
    }

    if include_casks {
        emit_log(
            app,
            state,
            "mac-brew",
            "Step 3: Upgrading Homebrew Casks (without --greedy)...",
            "info",
        );
        let cask_out = stream_command(
            app,
            state,
            "mac-brew",
            brew_command(&brew_path, &aug_path, &["upgrade", "--cask"]),
        )
        .await?;
        combined_upgrade_output.push_str(&cask_out.stdout);
        if cask_out.exit_code != 0 {
            warning_notes.push(format!("cask upgrade exited {}", cask_out.exit_code));
        }
    }

    if cleanup_after {
        emit_log(app, state, "mac-brew", "Step 4: Removing unused dependencies (autoremove)...", "info");
        let auto_out = stream_command(
            app,
            state,
            "mac-brew",
            brew_command(&brew_path, &aug_path, &["autoremove"]),
        )
        .await?;
        if auto_out.exit_code != 0 {
            warning_notes.push(format!("autoremove exited {}", auto_out.exit_code));
        }

        emit_log(app, state, "mac-brew", "Step 5: Cleaning Homebrew download cache...", "info");
        let clean_out = stream_command(
            app,
            state,
            "mac-brew",
            brew_command(&brew_path, &aug_path, &["cleanup"]),
        )
        .await?;
        if clean_out.exit_code != 0 {
            warning_notes.push(format!("cleanup exited {}", clean_out.exit_code));
        }
    }

    let (free_after, _) = get_disk_space();
    let freed = free_after.saturating_sub(free_before);
    let upgraded_count = parse_brew_upgraded_count(&combined_upgrade_output);

    if !warning_notes.is_empty() {
        return Ok(TaskResult {
            task_id: "mac-brew".to_string(),
            status: TaskStatus::Warning,
            summary: format!(
                "{} (warnings: {})",
                summarize_brew_run(upgraded_count, freed),
                warning_notes.join(", ")
            ),
            bytes_freed: (freed > 0).then_some(freed),
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    Ok(TaskResult {
        task_id: "mac-brew".to_string(),
        status: TaskStatus::Success,
        summary: summarize_brew_run(upgraded_count, freed),
        bytes_freed: (freed > 0).then_some(freed),
        reboot_required: false,
        open_settings_hint: false,
    })
}

#[cfg(target_os = "macos")]
fn remove_path_tree(
    path: &Path,
    state: &RunnerState,
    removed_items: &mut u64,
    removed_bytes: &mut u64,
    skipped_items: &mut u64,
) -> Result<(), RunError> {
    if state.is_canceled() {
        return Err(RunError::Canceled);
    }
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => {
            *skipped_items += 1;
            return Ok(());
        }
    };

    if meta.file_type().is_symlink() || meta.is_file() {
        let len = meta.len();
        match std::fs::remove_file(path) {
            Ok(()) => {
                *removed_items += 1;
                *removed_bytes = removed_bytes.saturating_add(len);
            }
            Err(_) => {
                *skipped_items += 1;
            }
        }
        return Ok(());
    }

    if meta.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(iter) => iter,
            Err(_) => {
                *skipped_items += 1;
                return Ok(());
            }
        };
        for entry in entries.flatten() {
            remove_path_tree(
                &entry.path(),
                state,
                removed_items,
                removed_bytes,
                skipped_items,
            )?;
        }
        if std::fs::remove_dir(path).is_ok() {
            *removed_items += 1;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
async fn run_mac_cleanup(
    app: &AppHandle,
    state: &RunnerState,
    options: &HashMap<String, bool>,
) -> Result<TaskResult, RunError> {
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| RunError::Message("Could not locate HOME directory.".to_string()))?;

    let empty_trash = options.get("empty_trash").copied().unwrap_or(false);
    let (free_before, _) = get_disk_space();

    let mut removed_items = 0u64;
    let mut removed_bytes = 0u64;
    let mut skipped_items = 0u64;

    let caches_dir = home.join("Library/Caches");
    if caches_dir.is_dir() {
        emit_log(
            app,
            state,
            "mac-cleanup",
            &format!("Scanning user caches in {}...", caches_dir.display()),
            "info",
        );
        if let Ok(entries) = std::fs::read_dir(&caches_dir) {
            for entry in entries.flatten() {
                if state.is_canceled() {
                    return Err(RunError::Canceled);
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if is_protected_mac_cache_name(&name) {
                    continue;
                }
                remove_path_tree(
                    &entry.path(),
                    state,
                    &mut removed_items,
                    &mut removed_bytes,
                    &mut skipped_items,
                )?;
            }
        }
    }

    let logs_dir = home.join("Library/Logs");
    if logs_dir.is_dir() {
        emit_log(
            app,
            state,
            "mac-cleanup",
            &format!("Scanning user logs in {}...", logs_dir.display()),
            "info",
        );
        if let Ok(entries) = std::fs::read_dir(&logs_dir) {
            for entry in entries.flatten() {
                if state.is_canceled() {
                    return Err(RunError::Canceled);
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if is_protected_mac_cache_name(&name) {
                    continue;
                }
                remove_path_tree(
                    &entry.path(),
                    state,
                    &mut removed_items,
                    &mut removed_bytes,
                    &mut skipped_items,
                )?;
            }
        }
    }

    emit_log(
        app,
        state,
        "mac-cleanup",
        &format!(
            "Cleaned {removed_items} cache/log items ({}); skipped {skipped_items} locked or protected items.",
            format_bytes(removed_bytes)
        ),
        "info",
    );

    if empty_trash {
        emit_log(app, state, "mac-cleanup", "Emptying Finder Trash...", "info");
        let mut trash_cmd = Command::new("osascript");
        trash_cmd.args(["-e", "tell application \"Finder\" to empty trash"]);
        let trash_out = stream_command(app, state, "mac-cleanup", trash_cmd).await?;
        if trash_out.exit_code != 0 {
            emit_log(
                app,
                state,
                "mac-cleanup",
                "Finder Trash could not be emptied (check Automation permission in System Settings).",
                "stderr",
            );
        }
    }

    let (free_after, _) = get_disk_space();
    let delta_freed = free_after.saturating_sub(free_before);
    let effective_freed = delta_freed.max(removed_bytes);

    let summary = if effective_freed > 0 {
        format!(
            "Removed {removed_items} items · Freed {}",
            format_bytes(effective_freed)
        )
    } else if removed_items > 0 {
        format!("Removed {removed_items} items.")
    } else {
        "User caches and logs are already clean.".to_string()
    };

    Ok(TaskResult {
        task_id: "mac-cleanup".to_string(),
        status: TaskStatus::Success,
        summary,
        bytes_freed: (effective_freed > 0).then_some(effective_freed),
        reboot_required: false,
        open_settings_hint: false,
    })
}

#[cfg(target_os = "macos")]
async fn run_mac_health(app: &AppHandle, state: &RunnerState) -> Result<TaskResult, RunError> {
    emit_log(app, state, "mac-health", "Verifying startup volume integrity...", "info");
    let mut verify_cmd = Command::new("/usr/sbin/diskutil");
    verify_cmd.args(["verifyVolume", "/"]);
    let verify_out = stream_command(app, state, "mac-health", verify_cmd).await?;

    let disk_ok = verify_out.exit_code == 0
        || verify_out.stdout.to_lowercase().contains("appears to be ok");

    emit_log(app, state, "mac-health", "Checking physical disk SMART status...", "info");
    let mut info_cmd = Command::new("/usr/sbin/diskutil");
    info_cmd.args(["info", "/"]);
    let info_out = stream_command(app, state, "mac-health", info_cmd).await?;

    let smart_line = info_out
        .stdout
        .lines()
        .find(|line| line.to_lowercase().contains("smart status"))
        .map(str::trim)
        .unwrap_or("SMART Status: Not reported on virtual/APFS volume");
    emit_log(app, state, "mac-health", smart_line, "info");

    let mut brew_warned = false;
    if let Some(brew_path) = detect_brew() {
        let aug_path =
            build_augmented_path(Some(&brew_path), std::env::var("PATH").ok().as_deref());
        emit_log(app, state, "mac-health", "Running brew doctor...", "info");
        let doc_out = stream_command(
            app,
            state,
            "mac-health",
            brew_command(&brew_path, &aug_path, &["doctor"]),
        )
        .await?;
        if classify_brew_doctor(doc_out.exit_code) == TaskStatus::Warning {
            brew_warned = true;
        }
    }

    if !disk_ok {
        return Ok(TaskResult {
            task_id: "mac-health".to_string(),
            status: TaskStatus::Failed,
            summary: "Startup volume verification reported an error. Review Technical Output."
                .to_string(),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    if brew_warned {
        return Ok(TaskResult {
            task_id: "mac-health".to_string(),
            status: TaskStatus::Warning,
            summary: "Startup disk verified OK · brew doctor reported warnings (see Technical Output)."
                .to_string(),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    Ok(TaskResult {
        task_id: "mac-health".to_string(),
        status: TaskStatus::Success,
        summary: "Startup disk and system health checks passed.".to_string(),
        bytes_freed: None,
        reboot_required: false,
        open_settings_hint: false,
    })
}

#[cfg(target_os = "macos")]
async fn run_mac_network(app: &AppHandle, state: &RunnerState) -> Result<TaskResult, RunError> {
    emit_log(
        app,
        state,
        "mac-network",
        "Requesting administrator authorization to flush DNS cache...",
        "info",
    );
    let out = run_macos_admin_script(
        app,
        state,
        "mac-network",
        "/usr/bin/dscacheutil -flushcache && /usr/bin/killall -HUP mDNSResponder",
    )
    .await?;

    if out.exit_code != 0 {
        return Ok(TaskResult {
            task_id: "mac-network".to_string(),
            status: TaskStatus::Failed,
            summary: format!("DNS flush failed with exit code {}.", out.exit_code),
            bytes_freed: None,
            reboot_required: false,
            open_settings_hint: false,
        });
    }

    emit_log(app, state, "mac-network", "DNS cache flushed and mDNSResponder refreshed.", "info");
    Ok(TaskResult {
        task_id: "mac-network".to_string(),
        status: TaskStatus::Success,
        summary: "DNS cache flushed.".to_string(),
        bytes_freed: None,
        reboot_required: false,
        open_settings_hint: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_apple_system_cache_prefixes() {
        assert!(is_protected_mac_cache_name("com.apple.Safari"));
        assert!(is_protected_mac_cache_name("CloudKit"));
        assert!(is_protected_mac_cache_name("GeoServices"));
        assert!(!is_protected_mac_cache_name("Homebrew"));
        assert!(!is_protected_mac_cache_name("Google"));
    }

    #[test]
    fn parses_brew_upgraded_package_count() {
        let log = "\
==> Upgrading 2 outdated packages:
node 22.0.0 -> 22.1.0
ripgrep 14.0.0 -> 14.1.0
==> Upgrading node
==> Upgrading ripgrep
";
        assert_eq!(parse_brew_upgraded_count(log), 2);
        assert_eq!(parse_brew_upgraded_count("Warning: already up-to-date."), 0);
    }

    #[test]
    fn maps_brew_doctor_nonzero_to_warning_not_failure() {
        assert_eq!(classify_brew_doctor(0), TaskStatus::Success);
        assert_eq!(classify_brew_doctor(1), TaskStatus::Warning);
    }

    #[test]
    fn detects_mac_update_settings_hint_and_reboot() {
        let res_settings = classify_mac_update_output(
            1,
            "Please open Software Update in System Settings to install.",
        );
        assert_eq!(res_settings.status, TaskStatus::Warning);
        assert!(res_settings.open_settings_hint);

        let res_ok = classify_mac_update_output(0, "Done. Please restart your computer.");
        assert_eq!(res_ok.status, TaskStatus::Success);
        assert!(res_ok.reboot_required);
    }
}
