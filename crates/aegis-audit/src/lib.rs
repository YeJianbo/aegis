use aegis_policy::RiskLevel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    NotRequired,
    Pending,
    Allowed,
    Denied,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub session_id: String,
    pub actor: ActorKind,
    pub actor_name: String,
    pub host_id: String,
    pub cwd: Option<String>,
    pub command: String,
    pub risk: RiskLevel,
    pub approval_status: ApprovalStatus,
    pub exit_code: Option<i32>,
    pub output_summary: Option<String>,
}

impl AuditEvent {
    pub fn new_agent_command(
        session_id: impl Into<String>,
        actor_name: impl Into<String>,
        host_id: impl Into<String>,
        command: impl Into<String>,
        risk: RiskLevel,
        approval_status: ApprovalStatus,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: session_id.into(),
            actor: ActorKind::Agent,
            actor_name: actor_name.into(),
            host_id: host_id.into(),
            cwd: None,
            command: command.into(),
            risk,
            approval_status,
            exit_code: None,
            output_summary: None,
        }
    }
}
