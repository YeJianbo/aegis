use std::{net::SocketAddr, sync::Arc};

use aegis_audit::AuditEvent;
use aegis_policy::{CommandAssessment, classify_command};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::RwLock};
use tracing::info;

#[derive(Clone, Default)]
struct AppState {
    audit_events: Arc<RwLock<Vec<AuditEvent>>>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, Deserialize)]
struct ClassifyCommandRequest {
    command: String,
}

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
        .route("/api/v1/policy/classify", post(classify))
        .route(
            "/api/v1/audit/events",
            get(list_audit_events).post(record_audit_event),
        )
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "aegis-gateway",
        status: "ok",
    })
}

async fn classify(Json(payload): Json<ClassifyCommandRequest>) -> Json<CommandAssessment> {
    Json(classify_command(payload.command))
}

async fn list_audit_events(State(state): State<AppState>) -> Json<Vec<AuditEvent>> {
    Json(state.audit_events.read().await.clone())
}

async fn record_audit_event(
    State(state): State<AppState>,
    Json(event): Json<AuditEvent>,
) -> Json<AuditEvent> {
    state.audit_events.write().await.push(event.clone());
    Json(event)
}
