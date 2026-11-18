use crate::models::{SystemInfo, TaskDef, TaskOptionDef};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            return meta.permissions().mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn detect_brew_in(candidates: &[PathBuf], path_env: Option<&str>) -> Option<PathBuf> {
    for candidate in candidates {
        if is_executable_file(candidate) {
            return Some(candidate.clone());
        }
    }

    if let Some(raw_path) = path_env {
        for dir in env::split_paths(raw_path) {
            let candidate = dir.join("brew");
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn detect_brew() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("/opt/homebrew/bin/brew"),
        PathBuf::from("/usr/local/bin/brew"),
    ];
    let path_var = env::var("PATH").ok();
    detect_brew_in(&candidates, path_var.as_deref())
}

pub fn build_augmented_path(brew_path: Option<&Path>, current_path: Option<&str>) -> String {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<OsString> = HashSet::new();

    let mut push_dir = |dir: PathBuf| {
        if dir.as_os_str().is_empty() {
            return;
        }
        let key = dir.as_os_str().to_os_string();
        if seen.insert(key) {
            dirs.push(dir);
        }
    };

    if let Some(brew) = brew_path {
        if let Some(parent) = brew.parent() {
            push_dir(parent.to_path_buf());
        }
    }

    for standard in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        push_dir(PathBuf::from(standard));
    }

    if let Some(raw) = current_path {
        for dir in env::split_paths(raw) {
            push_dir(dir);
        }
    }

    env::join_paths(&dirs)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            dirs.iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(":")
        })
}

pub fn parse_brew_version(stdout: &str) -> Option<String> {
    let first_line = stdout.lines().next()?.trim();
    let version = first_line
        .strip_prefix("Homebrew")
        .map(str::trim)
        .unwrap_or(first_line);
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

#[cfg(unix)]
pub fn get_disk_space() -> (u64, u64) {
    use std::mem::MaybeUninit;

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let res = unsafe { libc::statvfs(c"/".as_ptr(), stat.as_mut_ptr()) };
    if res != 0 {
        return (0, 0);
    }
    let stat = unsafe { stat.assume_init() };
    #[allow(clippy::unnecessary_cast)]
    let block_size = if stat.f_frsize > 0 {
        stat.f_frsize as u64
    } else {
        stat.f_bsize as u64
    };
    let free = u64::from(stat.f_bavail).saturating_mul(block_size);
    let total = u64::from(stat.f_blocks).saturating_mul(block_size);
    (free, total)
}

#[cfg(target_os = "windows")]
pub fn get_disk_space() -> (u64, u64) {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$d = Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='$env:SystemDrive'\"; if ($d) { \"$($d.FreeSpace) $($d.Size)\" }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut parts = text.split_whitespace();
            if let (Some(free_s), Some(total_s)) = (parts.next(), parts.next()) {
                if let (Ok(free), Ok(total)) = (free_s.parse::<u64>(), total_s.parse::<u64>()) {
                    return (free, total);
                }
            }
        }
    }
    (0, 0)
}

#[cfg(unix)]
pub fn is_admin() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(target_os = "windows")]
pub fn is_admin() -> bool {
    Command::new("whoami")
        .arg("/groups")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|out| {
            out.status.success()
                && String::from_utf8_lossy(&out.stdout).contains("S-1-16-12288")
        })
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn collect_platform_details() -> (String, String, Option<String>, Option<String>, bool, Option<String>) {
    let os_version = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_else(|| "macOS".to_string());

    let brew = detect_brew();
    let brew_path_str = brew.as_ref().map(|p| p.to_string_lossy().into_owned());
    let brew_version = brew.as_ref().and_then(|p| {
        let aug_path = build_augmented_path(Some(p), env::var("PATH").ok().as_deref());
        Command::new(p)
            .arg("--version")
            .env("PATH", aug_path)
            .env("HOMEBREW_NO_AUTO_UPDATE", "1")
            .env("HOMEBREW_NO_ENV_HINTS", "1")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| parse_brew_version(&String::from_utf8_lossy(&o.stdout)))
    });

    (
        "macos".to_string(),
        os_version,
        brew_path_str,
        brew_version,
        false,
        None,
    )
}

#[cfg(target_os = "windows")]
fn collect_platform_details() -> (String, String, Option<String>, Option<String>, bool, Option<String>) {
    let os_version = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$v = [System.Environment]::OSVersion.Version; if ($v.Build -ge 22000) { \"Windows 11 (Build $($v.Build))\" } else { \"Windows 10 (Build $($v.Build))\" }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Windows".to_string());

    let winget_output = Command::new("winget")
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|o| o.status.success());

    let winget_available = winget_output.is_some();
    let winget_version = winget_output.and_then(|o| {
        let ver = String::from_utf8_lossy(&o.stdout)
            .trim()
            .trim_start_matches(['v', 'V'])
            .to_string();
        if ver.is_empty() {
            None
        } else {
            Some(ver)
        }
    });

    (
        "windows".to_string(),
        os_version,
        None,
        None,
        winget_available,
        winget_version,
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn collect_platform_details() -> (String, String, Option<String>, Option<String>, bool, Option<String>) {
    (
        std::env::consts::OS.to_string(),
        std::env::consts::OS.to_string(),
        None,
        None,
        false,
        None,
    )
}

pub fn collect_system_info() -> SystemInfo {
    let (os, os_version, brew_path, brew_version, winget_available, winget_version) =
        collect_platform_details();
    let (free_disk_bytes, total_disk_bytes) = get_disk_space();

    SystemInfo {
        os,
        os_version,
        arch: env::consts::ARCH.to_string(),
        is_admin: is_admin(),
        free_disk_bytes,
        total_disk_bytes,
        brew_path,
        brew_version,
        winget_available,
        winget_version,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn build_task_list(info: &SystemInfo) -> Vec<TaskDef> {
    match info.os.as_str() {
        "macos" => {
            let has_brew = info.brew_path.is_some();
            let mut tasks = vec![TaskDef {
                id: "mac-update".to_string(),
                title: "Update macOS".to_string(),
                description: "Checks for and installs recommended macOS system updates.".to_string(),
                category: "updates".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            }];

            if has_brew {
                tasks.push(TaskDef {
                    id: "mac-brew".to_string(),
                    title: "Update Homebrew Packages".to_string(),
                    description: "Updates Homebrew, upgrades formulae and GUI apps, and cleans unused dependencies.".to_string(),
                    category: "packages".to_string(),
                    requires_admin: false,
                    badge: Some("Homebrew".to_string()),
                    available: true,
                    unavailable_reason: None,
                    options: vec![
                        TaskOptionDef {
                            key: "include_casks".to_string(),
                            label: "Include GUI apps (Casks)".to_string(),
                            description: "Upgrades Homebrew Casks without --greedy so self-updating apps stay untouched.".to_string(),
                            default_value: true,
                            requires_confirm: false,
                        },
                        TaskOptionDef {
                            key: "cleanup_after".to_string(),
                            label: "Autoremove & clean old downloads".to_string(),
                            description: "Runs brew autoremove and brew cleanup after upgrading.".to_string(),
                            default_value: true,
                            requires_confirm: false,
                        },
                    ],
                    default_selected: true,
                });
            }

            tasks.push(TaskDef {
                id: "mac-cleanup".to_string(),
                title: "Clean System Junk".to_string(),
                description: "Safely removes non-Apple user caches and old logs to reclaim disk space.".to_string(),
                category: "cleanup".to_string(),
                requires_admin: false,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![TaskOptionDef {
                    key: "empty_trash".to_string(),
                    label: "Empty Trash".to_string(),
                    description: "Permanently deletes items currently sitting in your Trash.".to_string(),
                    default_value: false,
                    requires_confirm: true,
                }],
                default_selected: true,
            });

            let health_desc = if has_brew {
                "Verifies startup disk integrity, checks drive SMART status, and runs brew doctor."
            } else {
                "Verifies startup disk integrity and checks drive SMART status."
            };
            tasks.push(TaskDef {
                id: "mac-health".to_string(),
                title: "Disk & System Health".to_string(),
                description: health_desc.to_string(),
                category: "health".to_string(),
                requires_admin: false,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            });

            tasks.push(TaskDef {
                id: "mac-network".to_string(),
                title: "Optimize Network".to_string(),
                description: "Flushes the macOS DNS cache and restarts mDNSResponder.".to_string(),
                category: "network".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            });

            tasks
        }
        "windows" => vec![
            TaskDef {
                id: "win-update".to_string(),
                title: "Update Windows".to_string(),
                description: "Searches for, downloads, and installs available Windows updates.".to_string(),
                category: "updates".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            },
            TaskDef {
                id: "win-winget".to_string(),
                title: "Update Software (Winget)".to_string(),
                description: "Upgrades installed desktop applications via Windows Package Manager.".to_string(),
                category: "packages".to_string(),
                requires_admin: false,
                badge: Some("Winget".to_string()),
                available: info.winget_available,
                unavailable_reason: (!info.winget_available).then(|| {
                    "Winget was not found in PATH. Install App Installer from Microsoft Store."
                        .to_string()
                }),
                options: vec![],
                default_selected: info.winget_available,
            },
            TaskDef {
                id: "win-cleanup".to_string(),
                title: "Clean System Junk".to_string(),
                description: "Clears Windows and user temporary files, Prefetch, and update download cache.".to_string(),
                category: "cleanup".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![TaskOptionDef {
                    key: "empty_recycle_bin".to_string(),
                    label: "Empty Recycle Bin".to_string(),
                    description: "Permanently deletes items currently in the Windows Recycle Bin.".to_string(),
                    default_value: false,
                    requires_confirm: true,
                }],
                default_selected: true,
            },
            TaskDef {
                id: "win-repair".to_string(),
                title: "Repair System Files".to_string(),
                description: "Runs DISM /RestoreHealth and System File Checker (sfc /scannow).".to_string(),
                category: "health".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            },
            TaskDef {
                id: "win-network".to_string(),
                title: "Optimize Network".to_string(),
                description: "Flushes DNS cache and resets Winsock and TCP/IP stack.".to_string(),
                category: "network".to_string(),
                requires_admin: true,
                badge: None,
                available: true,
                unavailable_reason: None,
                options: vec![],
                default_selected: true,
            },
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn sample_system(os: &str, brew_path: Option<&str>, winget: bool) -> SystemInfo {
        SystemInfo {
            os: os.to_string(),
            os_version: os.to_string(),
            arch: "aarch64".to_string(),
            is_admin: false,
            free_disk_bytes: 100,
            total_disk_bytes: 200,
            brew_path: brew_path.map(ToString::to_string),
            brew_version: brew_path.map(|_| "4.5.0".to_string()),
            winget_available: winget,
            winget_version: winget.then(|| "1.9.0".to_string()),
            app_version: "2.0.0".to_string(),
        }
    }

    #[test]
    fn detects_brew_from_candidates_and_path() {
        let temp_dir = env::temp_dir().join(format!("smt-test-brew-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let fake_brew = temp_dir.join("brew");
        fs::write(&fake_brew, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&fake_brew).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_brew, perms).unwrap();
        }

        let found_direct = detect_brew_in(std::slice::from_ref(&fake_brew), None);
        assert_eq!(found_direct, Some(fake_brew.clone()));

        let path_str = temp_dir.to_string_lossy().into_owned();
        let found_path = detect_brew_in(&[], Some(&path_str));
        assert_eq!(found_path, Some(fake_brew));

        let not_found = detect_brew_in(&[temp_dir.join("missing")], Some("/nonexistent-dir-xyz"));
        assert_eq!(not_found, None);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn builds_augmented_path_without_duplicates() {
        let brew = PathBuf::from("/opt/homebrew/bin/brew");
        let joined = build_augmented_path(Some(&brew), Some("/usr/bin:/custom/bin:/opt/homebrew/bin"));
        let parts: Vec<PathBuf> = env::split_paths(&joined).collect();

        assert_eq!(parts[0], PathBuf::from("/opt/homebrew/bin"));
        assert!(parts.contains(&PathBuf::from("/custom/bin")));
        let count_homebrew = parts
            .iter()
            .filter(|p| *p == &PathBuf::from("/opt/homebrew/bin"))
            .count();
        assert_eq!(count_homebrew, 1);
    }

    #[test]
    fn parses_brew_version_header() {
        assert_eq!(
            parse_brew_version("Homebrew 4.4.12\nHomebrew/homebrew-core"),
            Some("4.4.12".to_string())
        );
        assert_eq!(parse_brew_version(""), None);
    }

    #[test]
    fn filters_macos_brew_task_based_on_brew_presence() {
        let with_brew = build_task_list(&sample_system(
            "macos",
            Some("/opt/homebrew/bin/brew"),
            false,
        ));
        assert_eq!(with_brew.len(), 5);
        assert!(with_brew.iter().any(|t| t.id == "mac-brew"));
        let brew_task = with_brew.iter().find(|t| t.id == "mac-brew").unwrap();
        assert!(brew_task
            .options
            .iter()
            .any(|o| o.key == "include_casks" && o.default_value));

        let without_brew = build_task_list(&sample_system("macos", None, false));
        assert_eq!(without_brew.len(), 4);
        assert!(!without_brew.iter().any(|t| t.id == "mac-brew"));
    }

    #[test]
    fn builds_windows_task_list_and_winget_availability() {
        let with_winget = build_task_list(&sample_system("windows", None, true));
        assert_eq!(with_winget.len(), 5);
        assert!(with_winget.iter().find(|t| t.id == "win-winget").unwrap().available);

        let without_winget = build_task_list(&sample_system("windows", None, false));
        let winget_task = without_winget.iter().find(|t| t.id == "win-winget").unwrap();
        assert!(!winget_task.available);
        assert!(!winget_task.default_selected);
        assert!(winget_task.unavailable_reason.is_some());
    }
}
