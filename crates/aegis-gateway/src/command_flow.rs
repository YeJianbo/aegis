use aegis_audit::{ApprovalStatus, AuditEvent};
use aegis_policy::{CommandAssessment, RiskLevel, classify_command_with_rules};
use axum::http::StatusCode;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::{
    models::{
        ApprovalDecisionRequest, ApprovalRequest, CommandResponse, CommandStatus, PolicyConfig,
        PolicyMode, RunCommandRequest, SessionMode, SessionSummary, TerminalCommandStatus,
        TerminalCommandTask,
    },
    state::{AppState, GatewayState},
};

const COMMAND_WAIT_TIMEOUT_MS: u64 = 55_000;
const COMMAND_WAIT_INTERVAL_MS: u64 = 250;

#[derive(Debug)]
pub struct CommandFlowError {
    pub status: StatusCode,
    pub message: String,
}

impl CommandFlowError {
    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

pub async fn submit_command(
    state: AppState,
    payload: RunCommandRequest,
) -> Result<CommandResponse, CommandFlowError> {
    let actor_name = payload.actor_name.unwrap_or_else(|| "Codex".to_owned());
    let mut store = state.write().await;
    let assessment = classify_command_with_rules(&payload.command, &store.command_rules);
    let decision = evaluate_policy(&payload.command, assessment.clone(), &store.policy);
    let session = store
        .sessions
        .get(&payload.session_id)
        .cloned()
        .ok_or_else(|| CommandFlowError::not_found("session not found"))?;

    if let Some(reason) = decision.blocked_reason {
        let response = blocked_response(
            &mut store,
            &session,
            &actor_name,
            &payload.command,
            decision.assessment,
            &reason,
            &format!("blocked: {reason}"),
        );
        drop(store);
        persist_state(&state).await?;
        return Ok(response);
    }

    if session.paused {
        let response = blocked_response(
            &mut store,
            &session,
            &actor_name,
            &payload.command,
            decision.assessment,
            "Agent 已暂停，命令未执行",
            "blocked: agent paused",
        );
        drop(store);
        persist_state(&state).await?;
        return Ok(response);
    }

    if !matches!(session.mode, SessionMode::AgentWritable) {
        let response = blocked_response(
            &mut store,
            &session,
            &actor_name,
            &payload.command,
            decision.assessment,
            "当前会话不允许 Agent 写入",
            "blocked: session is not agent-writable",
        );
        drop(store);
        persist_state(&state).await?;
        return Ok(response);
    }

    if decision.assessment.approval_required {
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id: session.id.clone(),
            actor_name: actor_name.clone(),
            command: payload.command.clone(),
            assessment: decision.assessment.clone(),
            status: ApprovalStatus::Pending,
            proposed_command: None,
        };
        let event = audit_event(
            &session,
            &actor_name,
            &payload.command,
            decision.assessment.risk,
            ApprovalStatus::Pending,
            Some("命令等待人工审批".to_owned()),
        );
        store.approvals.insert(approval.id, approval.clone());
        store.audit_events.push(event.clone());
        let response = CommandResponse {
            status: CommandStatus::ApprovalPending,
            session_id: session.id,
            assessment: decision.assessment,
            approval_id: Some(approval.id),
            audit_event: event,
            output: None,
            terminal_command_id: None,
        };
        drop(store);
        persist_state(&state).await?;
        return Ok(response);
    }

    let task = enqueue_terminal_command(
        &mut store,
        &session,
        actor_name,
        payload.command,
        decision.assessment.risk,
        ApprovalStatus::NotRequired,
    );
    drop(store);
    persist_state(&state).await?;
    wait_for_terminal_command(state, task.id, session.id, decision.assessment, None).await
}

pub async fn decide_approval_command(
    state: AppState,
    approval_id: Uuid,
    payload: ApprovalDecisionRequest,
) -> Result<CommandResponse, CommandFlowError> {
    let mut store = state.write().await;
    let approval_snapshot = store
        .approvals
        .get(&approval_id)
        .cloned()
        .ok_or_else(|| CommandFlowError::not_found("approval not found"))?;
    let session = store
        .sessions
        .get(&approval_snapshot.session_id)
        .cloned()
        .ok_or_else(|| CommandFlowError::not_found("session not found"))?;

    let command = payload
        .modified_command
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| approval_snapshot.command.clone());
    let assessment = {
        let base = classify_command_with_rules(&command, &store.command_rules);
        evaluate_policy(&command, base, &store.policy).assessment
    };

    if !payload.allow {
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
        let response = CommandResponse {
            status: CommandStatus::Blocked,
            session_id: session.id,
            assessment,
            approval_id: Some(approval_id),
            audit_event: event,
            output: Some("denied by human".to_owned()),
            terminal_command_id: None,
        };
        drop(store);
        persist_state(&state).await?;
        return Ok(response);
    }

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

    let task = enqueue_terminal_command(
        &mut store,
        &session,
        approval_snapshot.actor_name,
        command,
        assessment.risk,
        approval_status,
    );
    drop(store);
    persist_state(&state).await?;
    wait_for_terminal_command(state, task.id, session.id, assessment, Some(approval_id)).await
}

pub(crate) struct PolicyDecision {
    pub assessment: CommandAssessment,
    pub blocked_reason: Option<String>,
}

async fn persist_state(state: &AppState) -> Result<(), CommandFlowError> {
    state.persist().await.map_err(|err| {
        CommandFlowError::internal(format!("failed to persist gateway state: {err}"))
    })
}

pub(crate) fn evaluate_policy(
    command: &str,
    mut assessment: CommandAssessment,
    policy: &PolicyConfig,
) -> PolicyDecision {
    let normalized = command.to_ascii_lowercase();
    if let Some(pattern) = find_pattern(&normalized, &policy.blacklist) {
        assessment.risk = RiskLevel::Critical;
        assessment.approval_required = false;
        assessment.reason = format!("命中命令黑名单: {pattern}");
        assessment.matched_rule = Some("policy-blacklist".to_owned());
        return PolicyDecision {
            assessment,
            blocked_reason: Some(format!("command blacklisted: {pattern}")),
        };
    }

    if let Some(pattern) = find_pattern(&normalized, &policy.whitelist) {
        assessment.approval_required = false;
        assessment.reason = format!("命中命令白名单: {pattern}");
        assessment.matched_rule = Some("policy-whitelist".to_owned());
        return PolicyDecision {
            assessment,
            blocked_reason: None,
        };
    }

    if matches!(policy.mode, PolicyMode::FullAllow) {
        assessment.approval_required = false;
        assessment.reason = format!("完全允许模式：{}", assessment.reason);
    }

    PolicyDecision {
        assessment,
        blocked_reason: None,
    }
}

fn find_pattern<'a>(normalized_command: &str, patterns: &'a [String]) -> Option<&'a str> {
    patterns
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .find(|pattern| normalized_command.contains(&pattern.to_ascii_lowercase()))
}

fn blocked_response(
    store: &mut GatewayState,
    session: &SessionSummary,
    actor_name: &str,
    command: &str,
    assessment: CommandAssessment,
    audit_summary: &str,
    output: &str,
) -> CommandResponse {
    let event = audit_event(
        session,
        actor_name,
        command,
        assessment.risk,
        ApprovalStatus::Denied,
        Some(audit_summary.to_owned()),
    );
    store.audit_events.push(event.clone());
    CommandResponse {
        status: CommandStatus::Blocked,
        session_id: session.id.clone(),
        assessment,
        approval_id: None,
        audit_event: event,
        output: Some(output.to_owned()),
        terminal_command_id: None,
    }
}

fn enqueue_terminal_command(
    store: &mut GatewayState,
    session: &SessionSummary,
    actor_name: String,
    command: String,
    risk: RiskLevel,
    approval_status: ApprovalStatus,
) -> TerminalCommandTask {
    let mut event = audit_event(
        session,
        &actor_name,
        &command,
        risk,
        approval_status.clone(),
        Some("命令已下发到桌面终端，等待真实执行".to_owned()),
    );
    event.exit_code = None;
    let task = TerminalCommandTask {
        id: Uuid::new_v4(),
        session_id: session.id.clone(),
        host_id: session.host_id.clone(),
        actor_name,
        command,
        risk,
        approval_status,
        status: TerminalCommandStatus::Pending,
        output: None,
        exit_code: None,
        error: None,
        tab_id: None,
        audit_event_id: event.id,
    };
    store.audit_events.push(event);
    store.terminal_commands.insert(task.id, task.clone());
    task
}

async fn wait_for_terminal_command(
    state: AppState,
    command_id: Uuid,
    session_id: String,
    assessment: CommandAssessment,
    approval_id: Option<Uuid>,
) -> Result<CommandResponse, CommandFlowError> {
    let started = std::time::Instant::now();
    loop {
        let maybe_response = {
            let store = state.read().await;
            let task = store
                .terminal_commands
                .get(&command_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("terminal command not found"))?;
            let event = store
                .audit_events
                .iter()
                .find(|event| event.id == task.audit_event_id)
                .cloned()
                .unwrap_or_else(|| {
                    audit_event(
                        &SessionSummary {
                            id: session_id.clone(),
                            host_id: task.host_id.clone(),
                            title: "Aegis Session".to_owned(),
                            cwd: None,
                            mode: SessionMode::AgentWritable,
                            paused: false,
                        },
                        &task.actor_name,
                        &task.command,
                        task.risk,
                        task.approval_status.clone(),
                        task.output.clone(),
                    )
                });

            match task.status {
                TerminalCommandStatus::Completed => Some(CommandResponse {
                    status: CommandStatus::Executed,
                    session_id: session_id.clone(),
                    assessment: assessment.clone(),
                    approval_id,
                    audit_event: event,
                    output: task.output,
                    terminal_command_id: Some(command_id),
                }),
                TerminalCommandStatus::Failed => Some(CommandResponse {
                    status: CommandStatus::Failed,
                    session_id: session_id.clone(),
                    assessment: assessment.clone(),
                    approval_id,
                    audit_event: event,
                    output: task.error.or(task.output),
                    terminal_command_id: Some(command_id),
                }),
                TerminalCommandStatus::Pending | TerminalCommandStatus::Running => None,
            }
        };

        if let Some(response) = maybe_response {
            return Ok(response);
        }

        if started.elapsed() >= Duration::from_millis(COMMAND_WAIT_TIMEOUT_MS) {
            let store = state.read().await;
            let task = store
                .terminal_commands
                .get(&command_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("terminal command not found"))?;
            let event = store
                .audit_events
                .iter()
                .find(|event| event.id == task.audit_event_id)
                .cloned()
                .ok_or_else(|| CommandFlowError::not_found("audit event not found"))?;
            return Ok(CommandResponse {
                status: CommandStatus::Queued,
                session_id,
                assessment,
                approval_id,
                audit_event: event,
                output: Some("queued: waiting for desktop terminal executor".to_owned()),
                terminal_command_id: Some(command_id),
            });
        }

        sleep(Duration::from_millis(COMMAND_WAIT_INTERVAL_MS)).await;
    }
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
