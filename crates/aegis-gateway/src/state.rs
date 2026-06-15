use std::{collections::HashMap, path::PathBuf, sync::Arc};

use aegis_audit::AuditEvent;
use aegis_policy::{CommandRule, default_rules, load_rules_from_yaml};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    models::{
        ApprovalRequest, FileTask, FileTaskStatus, HostSummary, PolicyConfig, SessionMode,
        SessionSummary, TerminalCommandStatus, TerminalCommandTask,
    },
    storage::{GatewaySnapshot, Storage},
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<GatewayState>>,
    storage: Option<Storage>,
}

#[derive(Debug)]
pub struct GatewayState {
    pub hosts: HashMap<String, HostSummary>,
    pub sessions: HashMap<String, SessionSummary>,
    pub approvals: HashMap<uuid::Uuid, ApprovalRequest>,
    pub terminal_commands: HashMap<uuid::Uuid, TerminalCommandTask>,
    pub file_tasks: HashMap<uuid::Uuid, FileTask>,
    pub audit_events: Vec<AuditEvent>,
    pub policy: PolicyConfig,
    pub command_rules: Vec<CommandRule>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new_in_memory(GatewayState::default())
    }
}

impl Default for GatewayState {
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
            hosts,
            sessions: HashMap::new(),
            approvals: HashMap::new(),
            terminal_commands: HashMap::new(),
            file_tasks: HashMap::new(),
            audit_events: Vec::new(),
            policy: PolicyConfig::default(),
            command_rules: default_rules(),
        }
    }
}

impl AppState {
    pub fn new_in_memory(state: GatewayState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(state)),
            storage: None,
        }
    }

    pub async fn from_env() -> anyhow::Result<Self> {
        if std::env::var("AEGIS_GATEWAY_DISABLE_DB").as_deref() == Ok("1") {
            let state = Self::default();
            state.load_command_rules_from_env().await;
            return Ok(state);
        }
        let db_path = std::env::var("AEGIS_GATEWAY_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("aegis-gateway.sqlite"));
        let state = Self::with_storage_path(db_path).await?;
        state.load_command_rules_from_env().await;
        Ok(state)
    }

    pub async fn with_storage_path(path: PathBuf) -> anyhow::Result<Self> {
        let storage = Storage::connect(path).await?;
        let state = match storage.load_snapshot().await? {
            Some(snapshot) => GatewayState::from(snapshot),
            None => GatewayState::default(),
        };
        Ok(Self {
            inner: Arc::new(RwLock::new(state)),
            storage: Some(storage),
        })
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GatewayState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, GatewayState> {
        self.inner.write().await
    }

    pub async fn persist(&self) -> anyhow::Result<()> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        let snapshot = self.read().await.snapshot();
        storage.save_snapshot(&snapshot).await
    }

    async fn load_command_rules_from_env(&self) {
        let Ok(path) = std::env::var("AEGIS_POLICY_RULES") else {
            return;
        };
        match load_rules_from_yaml(&path) {
            Ok(rules) if !rules.is_empty() => {
                let count = rules.len();
                self.write().await.command_rules = rules;
                info!(path, count, "loaded aegis command policy rules");
            }
            Ok(_) => {
                warn!(
                    path,
                    "policy rules file contained no valid rules; using defaults"
                );
            }
            Err(err) => {
                warn!(path, %err, "failed to load policy rules file; using defaults");
            }
        }
    }
}

impl GatewayState {
    pub fn snapshot(&self) -> GatewaySnapshot {
        GatewaySnapshot {
            hosts: self.hosts.values().cloned().collect(),
            sessions: self.sessions.values().cloned().collect(),
            approvals: self.approvals.values().cloned().collect(),
            terminal_commands: self.terminal_commands.values().cloned().collect(),
            file_tasks: self.file_tasks.values().cloned().collect(),
            audit_events: self.audit_events.clone(),
            policy: self.policy.clone(),
        }
    }
}

impl From<GatewaySnapshot> for GatewayState {
    fn from(snapshot: GatewaySnapshot) -> Self {
        let terminal_commands = snapshot.terminal_commands.into_iter().map(|mut task| {
            if matches!(task.status, TerminalCommandStatus::Running) {
                task.status = TerminalCommandStatus::Pending;
            }
            (task.id, task)
        });
        let file_tasks = snapshot.file_tasks.into_iter().map(|mut task| {
            if matches!(task.status, FileTaskStatus::Running) {
                task.status = FileTaskStatus::Pending;
            }
            (task.id, task)
        });
        Self {
            hosts: snapshot
                .hosts
                .into_iter()
                .map(|host| (host.id.clone(), host))
                .collect(),
            sessions: snapshot
                .sessions
                .into_iter()
                .map(|session| (session.id.clone(), session))
                .collect(),
            approvals: snapshot
                .approvals
                .into_iter()
                .map(|approval| (approval.id, approval))
                .collect(),
            terminal_commands: terminal_commands.collect(),
            file_tasks: file_tasks.collect(),
            audit_events: snapshot.audit_events,
            policy: snapshot.policy,
            command_rules: default_rules(),
        }
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

#[cfg(test)]
mod tests {
    use aegis_audit::{ApprovalStatus, AuditEvent};
    use aegis_policy::RiskLevel;
    use uuid::Uuid;

    use super::*;
    use crate::models::{PolicyMode, TerminalCommandTask};

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("aegis-gateway-test-{}.sqlite", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn persists_and_restores_gateway_snapshot() {
        let path = temp_db_path();
        let state = AppState::with_storage_path(path.clone()).await.unwrap();
        let session = new_session("demo-nginx".to_owned(), Some("Test".to_owned()));
        let event = AuditEvent::new_agent_command(
            &session.id,
            "Codex",
            &session.host_id,
            "git status",
            RiskLevel::Low,
            ApprovalStatus::NotRequired,
        );

        {
            let mut store = state.write().await;
            store.policy.mode = PolicyMode::FullAllow;
            store.sessions.insert(session.id.clone(), session.clone());
            store.audit_events.push(event.clone());
        }
        state.persist().await.unwrap();

        let restored = AppState::with_storage_path(path.clone()).await.unwrap();
        let store = restored.read().await;
        assert_eq!(store.policy.mode, PolicyMode::FullAllow);
        assert!(store.sessions.contains_key(&session.id));
        assert!(store.audit_events.iter().any(|item| item.id == event.id));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn restores_running_terminal_tasks_as_pending() {
        let session = new_session("demo-nginx".to_owned(), Some("Test".to_owned()));
        let task = TerminalCommandTask {
            id: Uuid::new_v4(),
            session_id: session.id.clone(),
            host_id: session.host_id.clone(),
            actor_name: "Codex".to_owned(),
            command: "pwd".to_owned(),
            risk: RiskLevel::Low,
            approval_status: ApprovalStatus::NotRequired,
            status: TerminalCommandStatus::Running,
            output: None,
            exit_code: None,
            error: None,
            tab_id: None,
            audit_event_id: Uuid::new_v4(),
        };
        let state = GatewayState::from(GatewaySnapshot {
            hosts: Vec::new(),
            sessions: vec![session],
            approvals: Vec::new(),
            terminal_commands: vec![task.clone()],
            file_tasks: Vec::new(),
            audit_events: Vec::new(),
            policy: PolicyConfig::default(),
        });

        assert!(matches!(
            state.terminal_commands.get(&task.id).unwrap().status,
            TerminalCommandStatus::Pending
        ));
    }
}
