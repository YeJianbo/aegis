use aegis_audit::{ApprovalStatus, AuditEvent};
use aegis_policy::{CommandAssessment, RiskLevel, classify_command};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    models::{
        ApprovalDecisionRequest, ApprovalRequest, CommandResponse, CommandStatus, HostSummary,
        OpenSessionRequest, RunCommandRequest, SessionSummary, SetSessionModeRequest,
        SyncHostsRequest,
    },
    state::{AppState, new_session},
};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, serde::Deserialize)]
pub struct ClassifyCommandRequest {
    command: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "aegis-gateway",
        status: "ok",
    })
}

pub async fn classify(Json(payload): Json<ClassifyCommandRequest>) -> Json<CommandAssessment> {
    Json(classify_command(payload.command))
}

pub async fn list_hosts(State(state): State<AppState>) -> Json<Vec<HostSummary>> {
    Json(state.read().await.hosts.values().cloned().collect())
}

pub async fn sync_hosts(
    State(state): State<AppState>,
    Json(payload): Json<SyncHostsRequest>,
) -> Json<Vec<HostSummary>> {
    let mut store = state.write().await;
    store.hosts = payload
        .hosts
        .into_iter()
        .filter(|host| !host.id.trim().is_empty() && !host.address.trim().is_empty())
        .map(|host| (host.id.clone(), host))
        .collect();
    Json(store.hosts.values().cloned().collect())
}

pub async fn open_session(
    State(state): State<AppState>,
    Json(payload): Json<OpenSessionRequest>,
) -> ApiResult<SessionSummary> {
    let mut store = state.write().await;
    if !store.hosts.contains_key(&payload.host_id) {
        return Err(not_found("host not found"));
    }

    let session = new_session(payload.host_id, payload.title);
    store.sessions.insert(session.id.clone(), session.clone());
    Ok(Json(session))
}

pub async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionSummary>> {
    Json(state.read().await.sessions.values().cloned().collect())
}

pub async fn pause_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<SessionSummary> {
    set_paused(state, session_id, true).await
}

pub async fn resume_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<SessionSummary> {
    set_paused(state, session_id, false).await
}

pub async fn set_session_mode(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(payload): Json<SetSessionModeRequest>,
) -> ApiResult<SessionSummary> {
    let mut store = state.write().await;
    let session = store
        .sessions
        .get_mut(&session_id)
        .ok_or_else(|| not_found("session not found"))?;
    session.mode = payload.mode.into();
    Ok(Json(session.clone()))
}

pub async fn run_command(
    State(state): State<AppState>,
    Json(payload): Json<RunCommandRequest>,
) -> ApiResult<CommandResponse> {
    let actor_name = payload.actor_name.unwrap_or_else(|| "Codex".to_owned());
    let assessment = classify_command(&payload.command);
    let mut store = state.write().await;
    let session = store
        .sessions
        .get(&payload.session_id)
        .cloned()
        .ok_or_else(|| not_found("session not found"))?;

    if session.paused {
        let event = audit_event(
            &session,
            &actor_name,
            &payload.command,
            assessment.risk,
            ApprovalStatus::Denied,
            Some("Agent 已暂停，命令未执行".to_owned()),
        );
        store.audit_events.push(event.clone());
        return Ok(Json(CommandResponse {
            status: CommandStatus::Blocked,
            session_id: session.id,
            assessment,
            approval_id: None,
            audit_event: event,
            output: Some("blocked: agent paused".to_owned()),
        }));
    }

    if !matches!(session.mode, crate::models::SessionMode::AgentWritable) {
        let event = audit_event(
            &session,
            &actor_name,
            &payload.command,
            assessment.risk,
            ApprovalStatus::Denied,
            Some("当前会话不允许 Agent 写入".to_owned()),
        );
        store.audit_events.push(event.clone());
        return Ok(Json(CommandResponse {
            status: CommandStatus::Blocked,
            session_id: session.id,
            assessment,
            approval_id: None,
            audit_event: event,
            output: Some("blocked: session is not agent-writable".to_owned()),
        }));
    }

    if assessment.approval_required {
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id: session.id.clone(),
            actor_name: actor_name.clone(),
            command: payload.command.clone(),
            assessment: assessment.clone(),
            status: ApprovalStatus::Pending,
            proposed_command: None,
        };
        let event = audit_event(
            &session,
            &actor_name,
            &payload.command,
            assessment.risk,
            ApprovalStatus::Pending,
            Some("命令等待人工审批".to_owned()),
        );
        store.approvals.insert(approval.id, approval.clone());
        store.audit_events.push(event.clone());
        return Ok(Json(CommandResponse {
            status: CommandStatus::ApprovalPending,
            session_id: session.id,
            assessment,
            approval_id: Some(approval.id),
            audit_event: event,
            output: None,
        }));
    }

    let output = simulated_terminal_output(&payload.command);
    let event = audit_event(
        &session,
        &actor_name,
        &payload.command,
        assessment.risk,
        ApprovalStatus::NotRequired,
        Some(output.clone()),
    );
    store.audit_events.push(event.clone());
    Ok(Json(CommandResponse {
        status: CommandStatus::Executed,
        session_id: session.id,
        assessment,
        approval_id: None,
        audit_event: event,
        output: Some(output),
    }))
}

pub async fn list_approvals(State(state): State<AppState>) -> Json<Vec<ApprovalRequest>> {
    Json(state.read().await.approvals.values().cloned().collect())
}

pub async fn decide_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<Uuid>,
    Json(payload): Json<ApprovalDecisionRequest>,
) -> ApiResult<CommandResponse> {
    let mut store = state.write().await;
    let approval_snapshot = store
        .approvals
        .get(&approval_id)
        .cloned()
        .ok_or_else(|| not_found("approval not found"))?;
    let session = store
        .sessions
        .get(&approval_snapshot.session_id)
        .cloned()
        .ok_or_else(|| not_found("session not found"))?;

    let command = payload
        .modified_command
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| approval_snapshot.command.clone());
    let assessment = classify_command(&command);

    if payload.allow {
        let approval_status = if command == approval_snapshot.command {
            ApprovalStatus::Allowed
        } else {
            ApprovalStatus::Modified
        };
        if let Some(approval) = store.approvals.get_mut(&approval_id) {
            approval.status = approval_status.clone();
            if matches!(approval_status, ApprovalStatus::Modified) {
                approval.proposed_command = Some(command.clone());
            }
        }
        let output = simulated_terminal_output(&command);
        let event = audit_event(
            &session,
            &approval_snapshot.actor_name,
            &command,
            assessment.risk,
            approval_status,
            Some(output.clone()),
        );
        store.audit_events.push(event.clone());
        return Ok(Json(CommandResponse {
            status: CommandStatus::Executed,
            session_id: session.id,
            assessment,
            approval_id: Some(approval_id),
            audit_event: event,
            output: Some(output),
        }));
    }

    if let Some(approval) = store.approvals.get_mut(&approval_id) {
        approval.status = ApprovalStatus::Denied;
    }
    let event = audit_event(
        &session,
        &approval_snapshot.actor_name,
        &command,
        assessment.risk,
        ApprovalStatus::Denied,
        Some("人工拒绝执行".to_owned()),
    );
    store.audit_events.push(event.clone());
    Ok(Json(CommandResponse {
        status: CommandStatus::Blocked,
        session_id: session.id,
        assessment,
        approval_id: Some(approval_id),
        audit_event: event,
        output: Some("denied by human".to_owned()),
    }))
}

pub async fn list_audit_events(State(state): State<AppState>) -> Json<Vec<AuditEvent>> {
    Json(state.read().await.audit_events.clone())
}

fn audit_event(
    session: &SessionSummary,
    actor_name: &str,
    command: &str,
    risk: RiskLevel,
    approval_status: ApprovalStatus,
    output_summary: Option<String>,
) -> AuditEvent {
    let mut event = AuditEvent::new_agent_command(
        &session.id,
        actor_name,
        &session.host_id,
        command,
        risk,
        approval_status,
    );
    event.cwd = session.cwd.clone();
    event.exit_code = Some(0);
    event.output_summary = output_summary;
    event
}

async fn set_paused(
    state: AppState,
    session_id: String,
    paused: bool,
) -> ApiResult<SessionSummary> {
    let mut store = state.write().await;
    let session = store
        .sessions
        .get_mut(&session_id)
        .ok_or_else(|| not_found("session not found"))?;
    session.paused = paused;
    Ok(Json(session.clone()))
}

fn simulated_terminal_output(command: &str) -> String {
    format!("[simulated terminal] executed: {command}")
}

fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}
