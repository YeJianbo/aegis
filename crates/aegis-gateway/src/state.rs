use std::{collections::HashMap, sync::Arc};

use aegis_audit::AuditEvent;
use tokio::sync::RwLock;

use crate::models::{
    ApprovalRequest, HostSummary, SessionMode, SessionSummary, TerminalCommandTask,
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<GatewayState>>,
}

#[derive(Debug)]
pub struct GatewayState {
    pub hosts: HashMap<String, HostSummary>,
    pub sessions: HashMap<String, SessionSummary>,
    pub approvals: HashMap<uuid::Uuid, ApprovalRequest>,
    pub terminal_commands: HashMap<uuid::Uuid, TerminalCommandTask>,
    pub audit_events: Vec<AuditEvent>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut hosts = HashMap::new();
        hosts.insert(
            "demo-nginx".to_owned(),
            HostSummary {
                id: "demo-nginx".to_owned(),
                name: "Demo Nginx Server".to_owned(),
                address: "127.0.0.1".to_owned(),
                tags: vec!["demo".to_owned(), "nginx".to_owned()],
            },
        );

        Self {
            inner: Arc::new(RwLock::new(GatewayState {
                hosts,
                sessions: HashMap::new(),
                approvals: HashMap::new(),
                terminal_commands: HashMap::new(),
                audit_events: Vec::new(),
            })),
        }
    }
}

impl AppState {
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GatewayState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, GatewayState> {
        self.inner.write().await
    }
}

pub fn new_session(host_id: String, title: Option<String>) -> SessionSummary {
    let id = uuid::Uuid::new_v4().to_string();
    SessionSummary {
        id,
        host_id,
        title: title.unwrap_or_else(|| "Aegis Session".to_owned()),
        cwd: None,
        mode: SessionMode::AgentWritable,
        paused: false,
    }
}
