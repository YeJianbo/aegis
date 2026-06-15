use std::net::SocketAddr;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tokio::net::TcpListener;
use tracing::info;

mod command_flow;
mod file_flow;
mod handlers;
mod mcp;
mod models;
mod state;
mod storage;

use handlers::{
    claim_file_task, claim_terminal_command, classify, close_session, complete_file_task,
    complete_terminal_command, decide_approval, export_audit_events_jsonl, get_policy_config,
    health, list_approvals, list_audit_events, list_hosts, list_sessions, open_session,
    pause_session, resume_session, run_command, run_file_operation, set_policy_config,
    set_session_mode, sync_hosts,
};
use mcp::{mcp_delete, mcp_endpoint, mcp_get};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aegis_gateway=info,tower_http=info".into()),
        )
        .init();

    let app = build_router(AppState::from_env().await?);
    let addr = SocketAddr::from(([127, 0, 0, 1], 17321));
    let listener = TcpListener::bind(addr).await?;

    info!(%addr, "aegis gateway listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/hosts", get(list_hosts))
        .route("/api/v1/hosts/sync", post(sync_hosts))
        .route("/api/v1/sessions", get(list_sessions).post(open_session))
        .route("/api/v1/sessions/{session_id}", delete(close_session))
        .route("/api/v1/sessions/{session_id}/pause", post(pause_session))
        .route("/api/v1/sessions/{session_id}/resume", post(resume_session))
        .route("/api/v1/sessions/{session_id}/mode", post(set_session_mode))
        .route("/api/v1/commands", post(run_command))
        .route("/api/v1/files", post(run_file_operation))
        .route("/api/v1/files/tasks/next", post(claim_file_task))
        .route(
            "/api/v1/files/tasks/{task_id}/complete",
            post(complete_file_task),
        )
        .route(
            "/api/v1/terminal/commands/next",
            post(claim_terminal_command),
        )
        .route(
            "/api/v1/terminal/commands/{command_id}/complete",
            post(complete_terminal_command),
        )
        .route("/api/v1/approvals", get(list_approvals))
        .route(
            "/api/v1/approvals/{approval_id}/decision",
            post(decide_approval),
        )
        .route("/api/v1/policy/classify", post(classify))
        .route(
            "/api/v1/policy",
            get(get_policy_config).put(set_policy_config),
        )
        .route("/api/v1/audit/events", get(list_audit_events))
        .route(
            "/api/v1/audit/events/export",
            get(export_audit_events_jsonl),
        )
        .route("/mcp", get(mcp_get).post(mcp_endpoint).delete(mcp_delete))
        .with_state(state)
}
