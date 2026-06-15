use aegis_audit::{ApprovalStatus, AuditEvent};
use aegis_policy::CommandAssessment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    AgentWritable,
    AgentReadOnly,
    HumanOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostSummary {
    pub id: String,
    pub name: String,
    pub address: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub host_id: String,
    pub title: String,
    pub cwd: Option<String>,
    pub mode: SessionMode,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Queued,
    Executed,
    ApprovalPending,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResponse {
    pub status: CommandStatus,
    pub session_id: String,
    pub assessment: CommandAssessment,
    pub approval_id: Option<Uuid>,
    pub audit_event: AuditEvent,
    pub output: Option<String>,
    pub terminal_command_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub session_id: String,
    pub actor_name: String,
    pub command: String,
    pub assessment: CommandAssessment,
    pub status: ApprovalStatus,
    pub proposed_command: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenSessionRequest {
    pub host_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncHostsRequest {
    pub hosts: Vec<HostSummary>,
}

#[derive(Debug, Deserialize)]
pub struct RunCommandRequest {
    pub session_id: String,
    pub command: String,
    pub actor_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalCommandStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCommandTask {
    pub id: Uuid,
    pub session_id: String,
    pub host_id: String,
    pub actor_name: String,
    pub command: String,
    pub risk: aegis_policy::RiskLevel,
    pub approval_status: ApprovalStatus,
    pub status: TerminalCommandStatus,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub tab_id: Option<String>,
    pub audit_event_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CompleteTerminalCommandRequest {
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Guarded,
    FullAllow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub mode: PolicyMode,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            mode: PolicyMode::Guarded,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyConfigRequest {
    pub mode: Option<PolicyMode>,
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperationKind {
    List,
    Stat,
    ReadFile,
    WriteFile,
    Delete,
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTask {
    pub id: Uuid,
    pub session_id: String,
    pub host_id: String,
    pub actor_name: String,
    pub operation: FileOperationKind,
    pub remote_path: Option<String>,
    pub local_path: Option<String>,
    pub content: Option<String>,
    pub status: FileTaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub tab_id: Option<String>,
    pub audit_event_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct FileOperationRequest {
    pub session_id: String,
    pub operation: FileOperationKind,
    pub remote_path: Option<String>,
    pub local_path: Option<String>,
    pub content: Option<String>,
    pub actor_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileOperationResponse {
    pub status: FileTaskStatus,
    pub task_id: Uuid,
    pub session_id: String,
    pub host_id: String,
    pub operation: FileOperationKind,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub audit_event: AuditEvent,
}

#[derive(Debug, Deserialize)]
pub struct CompleteFileTaskRequest {
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub tab_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalDecisionRequest {
    pub allow: bool,
    pub modified_command: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetSessionModeRequest {
    pub mode: SessionModeInput,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionModeInput {
    AgentWritable,
    AgentReadOnly,
    HumanOnly,
}

impl From<SessionModeInput> for SessionMode {
    fn from(value: SessionModeInput) -> Self {
        match value {
            SessionModeInput::AgentWritable => SessionMode::AgentWritable,
            SessionModeInput::AgentReadOnly => SessionMode::AgentReadOnly,
            SessionModeInput::HumanOnly => SessionMode::HumanOnly,
        }
    }
}
