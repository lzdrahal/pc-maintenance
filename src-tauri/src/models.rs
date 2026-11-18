use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub is_admin: bool,
    pub free_disk_bytes: u64,
    pub total_disk_bytes: u64,
    pub brew_path: Option<String>,
    pub brew_version: Option<String>,
    pub winget_available: bool,
    pub winget_version: Option<String>,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptionDef {
    pub key: String,
    pub label: String,
    pub description: String,
    pub default_value: bool,
    pub requires_confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDef {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub requires_admin: bool,
    pub badge: Option<String>,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub options: Vec<TaskOptionDef>,
    pub default_selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Success,
    Warning,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub summary: String,
    pub bytes_freed: Option<u64>,
    pub reboot_required: bool,
    pub open_settings_hint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutputEvent {
    pub task_id: String,
    pub line: String,
    pub stream: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: Option<String>,
}
