use std::net::SocketAddr;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tracing::info;

mod handlers;
mod mcp;
mod models;
mod state;

use handlers::{
    classify, decide_approval, health, list_approvals, list_audit_events, list_hosts,
    list_sessions, open_session, pause_session, resume_session, run_command, set_session_mode,
    sync_hosts,
};
use mcp::mcp_endpoint;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aegis_gateway=info,tower_http=info".into()),
        )
        .init();

    let app = build_router(AppState::default());
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
        .route("/api/v1/sessions/{session_id}/pause", post(pause_session))
        .route("/api/v1/sessions/{session_id}/resume", post(resume_session))
        .route("/api/v1/sessions/{session_id}/mode", post(set_session_mode))
        .route("/api/v1/commands", post(run_command))
        .route("/api/v1/approvals", get(list_approvals))
        .route(
            "/api/v1/approvals/{approval_id}/decision",
            post(decide_approval),
        )
        .route("/api/v1/policy/classify", post(classify))
        .route("/api/v1/audit/events", get(list_audit_events))
        .route("/mcp", post(mcp_endpoint))
        .with_state(state)
}
