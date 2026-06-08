use aegis_audit::{ApprovalStatus, AuditEvent};
use aegis_policy::CommandAssessment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    AgentWritable,
    AgentReadOnly,
    HumanOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostSummary {
    pub id: String,
    pub name: String,
    pub address: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub host_id: String,
    pub title: String,
    pub cwd: Option<String>,
    pub mode: SessionMode,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Executed,
    ApprovalPending,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandResponse {
    pub status: CommandStatus,
    pub session_id: String,
    pub assessment: CommandAssessment,
    pub approval_id: Option<Uuid>,
    pub audit_event: AuditEvent,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
pub struct RunCommandRequest {
    pub session_id: String,
    pub command: String,
    pub actor_name: Option<String>,
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
