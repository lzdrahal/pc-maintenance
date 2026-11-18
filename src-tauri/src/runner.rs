use crate::models::TaskOutputEvent;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Notify;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const MAX_SESSION_LOG_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum RunError {
    Canceled,
    Message(String),
}

impl From<RunError> for String {
    fn from(err: RunError) -> Self {
        match err {
            RunError::Canceled => "Task canceled by user.".to_string(),
            RunError::Message(msg) => msg,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct RunnerState {
    active_pid: Arc<Mutex<Option<u32>>>,
    canceled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    cancel_notify: Arc<Notify>,
    session_log: Arc<Mutex<String>>,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            active_pid: Arc::new(Mutex::new(None)),
            canceled: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            cancel_notify: Arc::new(Notify::new()),
            session_log: Arc::new(Mutex::new(String::new())),
        }
    }
}

pub struct ActiveRunGuard {
    active_pid: Arc<Mutex<Option<u32>>>,
    running: Arc<AtomicBool>,
}

impl Drop for ActiveRunGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.active_pid.lock() {
            *guard = None;
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

impl RunnerState {
    pub fn begin_run(&self) -> Result<ActiveRunGuard, String> {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Another maintenance task is already running.".to_string());
        }
        self.canceled.store(false, Ordering::SeqCst);
        if let Ok(mut pid_guard) = self.active_pid.lock() {
            *pid_guard = None;
        }
        Ok(ActiveRunGuard {
            active_pid: Arc::clone(&self.active_pid),
            running: Arc::clone(&self.running),
        })
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }

    pub fn set_active_pid(&self, pid: Option<u32>) {
        if let Ok(mut guard) = self.active_pid.lock() {
            *guard = pid;
        }
    }

    pub fn cancel_active(&self) {
        self.canceled.store(true, Ordering::SeqCst);
        let pid = self.active_pid.lock().ok().and_then(|g| *g);
        if let Some(pid) = pid {
            terminate_process_tree(pid);
        }
        self.cancel_notify.notify_waiters();
    }

    pub fn append_log(&self, entry: &str) {
        if let Ok(mut log) = self.session_log.lock() {
            log.push_str(entry);
            log.push('\n');
            if log.len() > MAX_SESSION_LOG_BYTES {
                let trim_from = log.len() - MAX_SESSION_LOG_BYTES;
                let safe_idx = log
                    .char_indices()
                    .find(|(idx, _)| *idx >= trim_from)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                log.drain(..safe_idx);
            }
        }
    }

    pub fn get_log(&self) -> String {
        self.session_log
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn clear_log(&self) {
        if let Ok(mut log) = self.session_log.lock() {
            log.clear();
        }
    }
}

#[cfg(unix)]
fn terminate_process_tree(pid: u32) {
    let pgid = -(pid as libc::pid_t);
    unsafe {
        libc::kill(pgid, libc::SIGTERM);
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        unsafe {
            libc::kill(pgid, libc::SIGKILL);
        }
    });
}

#[cfg(target_os = "windows")]
fn terminate_process_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

#[cfg(not(any(unix, target_os = "windows")))]
fn terminate_process_tree(_pid: u32) {}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c != '\x00' {
            out.push(c);
        }
    }
    out
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let val = bytes as f64;
    if val >= GB {
        format!("{:.2} GB", val / GB)
    } else if val >= MB {
        format!("{:.1} MB", val / MB)
    } else if val >= KB {
        format!("{:.0} KB", val / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn emit_log(app: &AppHandle, state: &RunnerState, task_id: &str, raw_line: &str, stream: &str) {
    let clean = strip_ansi(raw_line).trim_end_matches(['\r', '\n']).to_string();
    if clean.trim().is_empty() {
        return;
    }
    state.append_log(&format!("[{task_id}] {clean}"));
    let _ = app.emit(
        "task-output",
        TaskOutputEvent {
            task_id: task_id.to_string(),
            line: clean,
            stream: stream.to_string(),
        },
    );
}

fn split_progress_segments(raw_chunk: &str) -> Vec<String> {
    let clean = strip_ansi(raw_chunk);
    let parts: Vec<&str> = clean
        .split('\r')
        .map(str::trim_end)
        .filter(|s| !s.trim().is_empty())
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    // If a single line overwrote itself via \r progress bars, keep only the final status segment.
    if raw_chunk.contains('\r') && !raw_chunk.ends_with('\n') && parts.len() > 1 {
        return vec![parts[parts.len() - 1].to_string()];
    }
    if parts.len() > 3 && raw_chunk.contains('\r') {
        return vec![parts[parts.len() - 1].to_string()];
    }
    parts.into_iter().map(ToString::to_string).collect()
}

pub async fn stream_command(
    app: &AppHandle,
    state: &RunnerState,
    task_id: &str,
    mut cmd: Command,
) -> Result<CommandOutcome, RunError> {
    if state.is_canceled() {
        return Err(RunError::Canceled);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(unix)]
    cmd.process_group(0);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| RunError::Message(format!("Failed to start command: {e}")))?;

    let pid = child.id();
    state.set_active_pid(pid);

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_buf = Arc::new(Mutex::new(String::new()));
    let err_buf = Arc::new(Mutex::new(String::new()));

    let app_out = app.clone();
    let tid_out = task_id.to_string();
    let log_out = Arc::clone(&state.session_log);
    let out_buf_clone = Arc::clone(&out_buf);
    let stdout_handle = tokio::spawn(async move {
        if let Some(stream) = stdout {
            let mut reader = BufReader::new(stream);
            let mut raw = Vec::new();
            while let Ok(n) = reader.read_until(b'\n', &mut raw).await {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&raw).into_owned();
                raw.clear();
                for segment in split_progress_segments(&text) {
                    if let Ok(mut buf) = out_buf_clone.lock() {
                        buf.push_str(&segment);
                        buf.push('\n');
                    }
                    if let Ok(mut sess) = log_out.lock() {
                        sess.push_str(&format!("[{tid_out}] {segment}\n"));
                    }
                    let _ = app_out.emit(
                        "task-output",
                        TaskOutputEvent {
                            task_id: tid_out.clone(),
                            line: segment,
                            stream: "stdout".to_string(),
                        },
                    );
                }
            }
        }
    });

    let app_err = app.clone();
    let tid_err = task_id.to_string();
    let log_err = Arc::clone(&state.session_log);
    let err_buf_clone = Arc::clone(&err_buf);
    let stderr_handle = tokio::spawn(async move {
        if let Some(stream) = stderr {
            let mut reader = BufReader::new(stream);
            let mut raw = Vec::new();
            while let Ok(n) = reader.read_until(b'\n', &mut raw).await {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&raw).into_owned();
                raw.clear();
                for segment in split_progress_segments(&text) {
                    if let Ok(mut buf) = err_buf_clone.lock() {
                        buf.push_str(&segment);
                        buf.push('\n');
                    }
                    if let Ok(mut sess) = log_err.lock() {
                        sess.push_str(&format!("[{tid_err}] {segment}\n"));
                    }
                    let _ = app_err.emit(
                        "task-output",
                        TaskOutputEvent {
                            task_id: tid_err.clone(),
                            line: segment,
                            stream: "stderr".to_string(),
                        },
                    );
                }
            }
        }
    });

    let wait_res = tokio::select! {
        status = child.wait() => Some(status),
        _ = state.cancel_notify.notified() => None,
    };

    if wait_res.is_none() || state.is_canceled() {
        if let Some(id) = pid {
            terminate_process_tree(id);
        }
        let _ = child.kill().await;
        let _ = child.wait().await;
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;
        state.set_active_pid(None);
        return Err(RunError::Canceled);
    }

    let status = wait_res
        .unwrap()
        .map_err(|e| RunError::Message(format!("Process wait error: {e}")))?;

    let _ = stdout_handle.await;
    let _ = stderr_handle.await;
    state.set_active_pid(None);

    let collected_out = out_buf.lock().map(|s| s.clone()).unwrap_or_default();
    let collected_err = err_buf.lock().map(|s| s.clone()).unwrap_or_default();

    Ok(CommandOutcome {
        exit_code: status.code().unwrap_or(-1),
        stdout: collected_out,
        stderr: collected_err,
    })
}

#[cfg(target_os = "macos")]
pub async fn run_macos_admin_script(
    app: &AppHandle,
    state: &RunnerState,
    task_id: &str,
    shell_cmd: &str,
) -> Result<CommandOutcome, RunError> {
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    if state.is_canceled() {
        return Err(RunError::Canceled);
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("smt-admin-{}-{nonce}", std::process::id()));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&temp_dir)
        .map_err(|e| RunError::Message(format!("Failed to create secure temp directory: {e}")))?;

    let log_path = temp_dir.join("output.log");
    let _ = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o644)
        .open(&log_path)
        .map_err(|e| {
            let _ = fs::remove_dir_all(&temp_dir);
            RunError::Message(format!("Failed to prepare admin log file: {e}"))
        })?;

    let log_path_str = log_path.to_string_lossy().into_owned();
    if log_path_str.contains('\'') || log_path_str.contains('"') || log_path_str.contains('\n') {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(RunError::Message(
            "Temporary directory path contains unsupported quote characters.".to_string(),
        ));
    }

    let full_shell = format!("{shell_cmd} > '{log_path_str}' 2>&1");
    let escaped_for_applescript = full_shell.replace('\\', "\\\\").replace('"', "\\\"");
    let applescript =
        format!("do shell script \"{escaped_for_applescript}\" with administrator privileges");

    let mut cmd = Command::new("osascript");
    cmd.args(["-e", &applescript])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = cmd.spawn().map_err(|e| {
        let _ = fs::remove_dir_all(&temp_dir);
        RunError::Message(format!("Failed to launch macOS authorization prompt: {e}"))
    })?;

    let pid = child.id();
    state.set_active_pid(pid);

    let mut tail_offset: u64 = 0;
    let mut partial_line = String::new();
    let mut collected_out = String::new();
    let mut interval = tokio::time::interval(Duration::from_millis(150));

    let flush_tail = |offset: &mut u64, partial: &mut String, out_acc: &mut String| {
        if let Ok(mut file) = File::open(&log_path) {
            if file.seek(SeekFrom::Start(*offset)).is_ok() {
                let mut chunk = Vec::new();
                if let Ok(read_bytes) = file.read_to_end(&mut chunk) {
                    if read_bytes > 0 {
                        *offset += read_bytes as u64;
                        let text = String::from_utf8_lossy(&chunk);
                        partial.push_str(&text);
                        while let Some(pos) = partial.find('\n') {
                            let line = partial[..pos].to_string();
                            partial.drain(..=pos);
                            let clean = strip_ansi(&line).trim_end_matches('\r').to_string();
                            if !clean.trim().is_empty() {
                                out_acc.push_str(&clean);
                                out_acc.push('\n');
                                emit_log(app, state, task_id, &clean, "stdout");
                            }
                        }
                    }
                }
            }
        }
    };

    let status = loop {
        tokio::select! {
            _ = interval.tick() => {
                flush_tail(&mut tail_offset, &mut partial_line, &mut collected_out);
                if let Ok(Some(status)) = child.try_wait() {
                    break status;
                }
            }
            _ = state.cancel_notify.notified() => {
                if let Some(id) = pid {
                    terminate_process_tree(id);
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
                state.set_active_pid(None);
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(RunError::Canceled);
            }
        }
    };

    flush_tail(&mut tail_offset, &mut partial_line, &mut collected_out);
    if !partial_line.trim().is_empty() {
        let clean = strip_ansi(&partial_line).trim_end_matches('\r').to_string();
        collected_out.push_str(&clean);
        collected_out.push('\n');
        emit_log(app, state, task_id, &clean, "stdout");
    }

    let mut osascript_err = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        if stderr.read_to_end(&mut buf).await.is_ok() {
            osascript_err = String::from_utf8_lossy(&buf).trim().to_string();
        }
    }

    state.set_active_pid(None);
    let _ = fs::remove_dir_all(&temp_dir);

    if state.is_canceled() {
        return Err(RunError::Canceled);
    }

    if !status.success()
        && (osascript_err.contains("User canceled") || osascript_err.contains("(-128)"))
    {
        return Err(RunError::Canceled);
    }

    if !osascript_err.is_empty() {
        emit_log(app, state, task_id, &osascript_err, "stderr");
    }

    Ok(CommandOutcome {
        exit_code: status.code().unwrap_or(-1),
        stdout: collected_out,
        stderr: osascript_err,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[32mSuccess\x1b[0m!"), "Success!");
        assert_eq!(strip_ansi("\x1b[1;36m==> Upgrading\x1b[0m"), "==> Upgrading");
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn formats_byte_sizes_human_readably() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024 / 2), "1.50 GB");
    }
}
