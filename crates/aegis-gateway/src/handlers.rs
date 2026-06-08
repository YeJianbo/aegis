use aegis_audit::AuditEvent;
use aegis_policy::{CommandAssessment, classify_command};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    command_flow::{CommandFlowError, decide_approval_command, submit_command},
    models::{
        ApprovalDecisionRequest, ApprovalRequest, CommandResponse, CompleteTerminalCommandRequest,
        HostSummary, OpenSessionRequest, RunCommandRequest, SessionSummary, SetSessionModeRequest,
        SyncHostsRequest, TerminalCommandStatus, TerminalCommandTask,
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
    submit_command(state, payload)
        .await
        .map(Json)
        .map_err(flow_error)
}

pub async fn list_approvals(State(state): State<AppState>) -> Json<Vec<ApprovalRequest>> {
    Json(state.read().await.approvals.values().cloned().collect())
}

pub async fn decide_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<Uuid>,
    Json(payload): Json<ApprovalDecisionRequest>,
) -> ApiResult<CommandResponse> {
    decide_approval_command(state, approval_id, payload)
        .await
        .map(Json)
        .map_err(flow_error)
}

pub async fn list_audit_events(State(state): State<AppState>) -> Json<Vec<AuditEvent>> {
    Json(state.read().await.audit_events.clone())
}

pub async fn claim_terminal_command(
    State(state): State<AppState>,
) -> Json<Option<TerminalCommandTask>> {
    let mut store = state.write().await;
    let task = store
        .terminal_commands
        .values_mut()
        .find(|task| matches!(task.status, TerminalCommandStatus::Pending));
    if let Some(task) = task {
        task.status = TerminalCommandStatus::Running;
        return Json(Some(task.clone()));
    }
    Json(None)
}

pub async fn complete_terminal_command(
    State(state): State<AppState>,
    Path(command_id): Path<Uuid>,
    Json(payload): Json<CompleteTerminalCommandRequest>,
) -> ApiResult<TerminalCommandTask> {
    let mut store = state.write().await;
    let task = store
        .terminal_commands
        .get_mut(&command_id)
        .ok_or_else(|| not_found("terminal command not found"))?;
    task.output = payload.output.clone();
    task.exit_code = payload.exit_code;
    task.error = payload.error.clone();
    task.tab_id = payload.tab_id.clone();
    task.status = if payload
        .error
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        TerminalCommandStatus::Failed
    } else {
        TerminalCommandStatus::Completed
    };
    let completed = task.clone();

    if let Some(event) = store
        .audit_events
        .iter_mut()
        .find(|event| event.id == completed.audit_event_id)
    {
        event.exit_code = completed.exit_code;
        event.output_summary = completed
            .error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| completed.output.clone());
    }

    Ok(Json(completed))
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

fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn flow_error(err: CommandFlowError) -> (StatusCode, Json<ErrorResponse>) {
    (err.status, Json(ErrorResponse { error: err.message }))
}
