use std::path::Path;

use aegis_audit::AuditEvent;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::models::{
    ApprovalRequest, FileTask, HostSummary, PolicyConfig, SessionSummary, TerminalCommandTask,
};

const SNAPSHOT_KEY: &str = "gateway_state";

#[derive(Debug, Clone)]
pub struct Storage {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySnapshot {
    pub hosts: Vec<HostSummary>,
    pub sessions: Vec<SessionSummary>,
    pub approvals: Vec<ApprovalRequest>,
    pub terminal_commands: Vec<TerminalCommandTask>,
    pub file_tasks: Vec<FileTask>,
    pub audit_events: Vec<AuditEvent>,
    pub policy: PolicyConfig,
}

impl Storage {
    pub async fn connect(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create gateway data dir {}", parent.display())
            })?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to open gateway database {}", path.display()))?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    pub async fn load_snapshot(&self) -> anyhow::Result<Option<GatewaySnapshot>> {
        let row = sqlx::query("select value from state_snapshots where key = ?")
            .bind(SNAPSHOT_KEY)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let value: String = row.try_get("value")?;
        let snapshot = serde_json::from_str(&value).context("failed to decode gateway snapshot")?;
        Ok(Some(snapshot))
    }

    pub async fn save_snapshot(&self, snapshot: &GatewaySnapshot) -> anyhow::Result<()> {
        let value = serde_json::to_string(snapshot).context("failed to encode gateway snapshot")?;
        sqlx::query(
            "insert into state_snapshots (key, value, updated_at) values (?, ?, unixepoch())
             on conflict(key) do update set value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(SNAPSHOT_KEY)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            "create table if not exists state_snapshots (
                key text primary key,
                value text not null,
                updated_at integer not null
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
