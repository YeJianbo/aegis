use aegis_audit::{ApprovalStatus, AuditEvent};
use aegis_policy::RiskLevel;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::{
    command_flow::CommandFlowError,
    models::{
        FileOperationKind, FileOperationRequest, FileOperationResponse, FileTask, FileTaskStatus,
        SessionMode, SessionSummary,
    },
    state::{AppState, GatewayState},
};

const FILE_WAIT_TIMEOUT_MS: u64 = 55_000;
const FILE_WAIT_INTERVAL_MS: u64 = 250;

pub async fn submit_file_operation(
    state: AppState,
    payload: FileOperationRequest,
) -> Result<FileOperationResponse, CommandFlowError> {
    let actor_name = payload
        .actor_name
        .clone()
        .unwrap_or_else(|| "Codex".to_owned());
    let mut store = state.write().await;
    let session = store
        .sessions
        .get(&payload.session_id)
        .cloned()
        .ok_or_else(|| CommandFlowError::not_found("session not found"))?;

    if session.paused {
        return Ok(blocked_file_response(
            &mut store,
            &session,
            &actor_name,
            payload.operation,
            "Agent 已暂停，文件操作未执行",
            "blocked: agent paused",
        ));
    }

    if matches!(session.mode, SessionMode::HumanOnly) {
        return Ok(blocked_file_response(
            &mut store,
            &session,
            &actor_name,
            payload.operation,
            "当前会话由人工接管，文件操作未执行",
            "blocked: session is human-only",
        ));
    }

    if matches!(session.mode, SessionMode::AgentReadOnly)
        && matches!(
            &payload.operation,
            FileOperationKind::Upload | FileOperationKind::Delete | FileOperationKind::WriteFile
        )
    {
        return Ok(blocked_file_response(
            &mut store,
            &session,
            &actor_name,
            payload.operation,
            "只读模式禁止写入远程文件",
            "blocked: session is read-only",
        ));
    }

    validate_file_payload(&payload)?;
    let task = enqueue_file_task(&mut store, &session, actor_name, payload);
    drop(store);
    wait_for_file_task(state, task.id).await
}

fn validate_file_payload(payload: &FileOperationRequest) -> Result<(), CommandFlowError> {
    match &payload.operation {
        FileOperationKind::List
        | FileOperationKind::Stat
        | FileOperationKind::ReadFile
        | FileOperationKind::WriteFile
        | FileOperationKind::Delete => {
            if payload
                .remote_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err(CommandFlowError::bad_request("remote_path is required"));
            }
            if matches!(&payload.operation, FileOperationKind::WriteFile)
                && payload.content.is_none()
            {
                return Err(CommandFlowError::bad_request("content is required"));
            }
        }
        FileOperationKind::Upload => {
            require_path(payload.local_path.as_deref(), "local_path")?;
            require_path(payload.remote_path.as_deref(), "remote_path")?;
        }
        FileOperationKind::Download => {
            require_path(payload.remote_path.as_deref(), "remote_path")?;
            require_path(payload.local_path.as_deref(), "local_path")?;
        }
    }
    Ok(())
}

fn require_path(value: Option<&str>, name: &str) -> Result<(), CommandFlowError> {
    if value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(CommandFlowError::bad_request(format!("{name} is required")));
    }
    Ok(())
}

fn enqueue_file_task(
    store: &mut GatewayState,
    session: &SessionSummary,
    actor_name: String,
    payload: FileOperationRequest,
) -> FileTask {
    let command = file_audit_command(
        &payload.operation,
        payload.remote_path.as_deref(),
        payload.local_path.as_deref(),
    );
    let mut event = audit_event(
        session,
        &actor_name,
        &command,
        ApprovalStatus::NotRequired,
        Some("文件操作已下发到桌面端，等待真实执行".to_owned()),
    );
    event.exit_code = None;
    let task = FileTask {
        id: Uuid::new_v4(),
        session_id: session.id.clone(),
        host_id: session.host_id.clone(),
        actor_name,
        operation: payload.operation,
        remote_path: payload.remote_path,
        local_path: payload.local_path,
        content: payload.content,
        status: FileTaskStatus::Pending,
        result: None,
        error: None,
        tab_id: None,
        audit_event_id: event.id,
    };
    store.audit_events.push(event);
    store.file_tasks.insert(task.id, task.clone());
    task
}

async fn wait_for_file_task(
    state: AppState,
    task_id: Uuid,
) -> Result<FileOperationResponse, CommandFlowError> {
    let started = std::time::Instant::now();
    loop {
        let maybe_response = {
            let store = state.read().await;
            let task = store
                .file_tasks
                .get(&task_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("file task not found"))?;
            let event = store
                .audit_events
                .iter()
                .find(|event| event.id == task.audit_event_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("audit event not found"))?;
            match task.status {
                FileTaskStatus::Completed | FileTaskStatus::Failed => Some(FileOperationResponse {
                    status: task.status,
                    task_id: task.id,
                    session_id: task.session_id,
                    host_id: task.host_id,
                    operation: task.operation,
                    result: task.result,
                    error: task.error,
                    audit_event: event,
                }),
                FileTaskStatus::Pending | FileTaskStatus::Running => None,
            }
        };

        if let Some(response) = maybe_response {
            return Ok(response);
        }

        if started.elapsed() >= Duration::from_millis(FILE_WAIT_TIMEOUT_MS) {
            let store = state.read().await;
            let task = store
                .file_tasks
                .get(&task_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("file task not found"))?;
            let event = store
                .audit_events
                .iter()
                .find(|event| event.id == task.audit_event_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("audit event not found"))?;
            return Ok(FileOperationResponse {
                status: FileTaskStatus::Pending,
                task_id: task.id,
                session_id: task.session_id,
                host_id: task.host_id,
                operation: task.operation,
                result: None,
                error: Some("queued: waiting for desktop file executor".to_owned()),
                audit_event: event,
            });
        }

        sleep(Duration::from_millis(FILE_WAIT_INTERVAL_MS)).await;
    }
}

fn blocked_file_response(
    store: &mut GatewayState,
    session: &SessionSummary,
    actor_name: &str,
    operation: FileOperationKind,
    audit_summary: &str,
    error: &str,
) -> FileOperationResponse {
    let command = file_audit_command(&operation, None, None);
    let event = audit_event(
        session,
        actor_name,
        &command,
        ApprovalStatus::Denied,
        Some(audit_summary.to_owned()),
    );
    store.audit_events.push(event.clone());
    FileOperationResponse {
        status: FileTaskStatus::Failed,
        task_id: Uuid::new_v4(),
        session_id: session.id.clone(),
        host_id: session.host_id.clone(),
        operation,
        result: None,
        error: Some(error.to_owned()),
        audit_event: event,
    }
}

fn file_audit_command(
    operation: &FileOperationKind,
    remote_path: Option<&str>,
    local_path: Option<&str>,
) -> String {
    let op = match operation {
        FileOperationKind::List => "file:list",
        FileOperationKind::Stat => "file:stat",
        FileOperationKind::ReadFile => "file:read",
        FileOperationKind::WriteFile => "file:write",
        FileOperationKind::Delete => "file:delete",
        FileOperationKind::Upload => "file:upload",
        FileOperationKind::Download => "file:download",
    };
    [Some(op), remote_path, local_path]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

fn audit_event(
    session: &SessionSummary,
    actor_name: &str,
    command: &str,
    approval_status: ApprovalStatus,
    output_summary: Option<String>,
) -> AuditEvent {
    let mut event = AuditEvent::new_agent_command(
        &session.id,
        actor_name,
        &session.host_id,
        command,
        RiskLevel::Low,
        approval_status,
    );
    event.cwd = session.cwd.clone();
    event.exit_code = Some(0);
    event.output_summary = output_summary;
    event
}
