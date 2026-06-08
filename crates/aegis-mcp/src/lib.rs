use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub gateway_endpoint: String,
}

pub fn default_tools() -> Vec<ToolDescriptor> {
    vec![
        tool(
            "list_hosts",
            "列出当前用户可见的远程主机",
            "/api/v1/hosts",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "open_session",
            "打开或复用一个受控远程终端会话",
            "/api/v1/sessions",
            json!({
                "type": "object",
                "required": ["host_id"],
                "properties": {
                    "host_id": { "type": "string" },
                    "title": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "run_command",
            "在受控会话中执行命令并返回输出；高危命令会返回审批状态",
            "/api/v1/commands",
            json!({
                "type": "object",
                "required": ["session_id", "command"],
                "properties": {
                    "session_id": { "type": "string" },
                    "command": { "type": "string" },
                    "actor_name": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "close_session",
            "关闭 Aegis 受控会话并清理未完成任务",
            "/api/v1/sessions/{session_id}",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "tail_log",
            "读取远程日志尾部内容，MVP 阶段映射为受控 tail 命令",
            "/api/v1/commands",
            json!({
                "type": "object",
                "required": ["session_id", "path"],
                "properties": {
                    "session_id": { "type": "string" },
                    "path": { "type": "string" },
                    "lines": { "type": "integer", "minimum": 1, "maximum": 1000 }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "get_terminal_snapshot",
            "获取终端当前可见快照",
            "/api/v1/sessions",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" },
                    "lines": { "type": "integer", "minimum": 1, "maximum": 500 }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "get_policy",
            "读取当前 Aegis 命令策略配置",
            "/api/v1/policy",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "update_policy",
            "更新 Aegis 命令策略配置，支持 guarded/full_allow、白名单和黑名单",
            "/api/v1/policy",
            json!({
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["guarded", "full_allow"] },
                    "whitelist": { "type": "array", "items": { "type": "string" } },
                    "blacklist": { "type": "array", "items": { "type": "string" } }
                },
                "additionalProperties": false
            }),
        ),
        file_tool("file_list", "列出远程 SFTP/FTP 目录", "list"),
        file_tool("file_stat", "读取远程 SFTP/FTP 文件或目录状态", "stat"),
        file_tool("file_read", "读取远程 SFTP/FTP 文件内容", "read_file"),
        tool(
            "file_write",
            "写入远程 SFTP/FTP 文本文件内容",
            "/api/v1/files",
            json!({
                "type": "object",
                "required": ["session_id", "remote_path", "content"],
                "properties": {
                    "session_id": { "type": "string" },
                    "remote_path": { "type": "string" },
                    "content": { "type": "string" },
                    "actor_name": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        file_tool("file_delete", "删除远程 SFTP/FTP 文件或目录", "delete"),
        tool(
            "file_upload",
            "上传本地文件或目录到远程 SFTP/FTP 路径",
            "/api/v1/files",
            json!({
                "type": "object",
                "required": ["session_id", "local_path", "remote_path"],
                "properties": {
                    "session_id": { "type": "string" },
                    "local_path": { "type": "string" },
                    "remote_path": { "type": "string" },
                    "actor_name": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "file_download",
            "下载远程 SFTP/FTP 文件或目录到本地路径",
            "/api/v1/files",
            json!({
                "type": "object",
                "required": ["session_id", "remote_path", "local_path"],
                "properties": {
                    "session_id": { "type": "string" },
                    "remote_path": { "type": "string" },
                    "local_path": { "type": "string" },
                    "actor_name": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum ToolCall {
    ListHosts,
    OpenSession {
        host_id: String,
        title: Option<String>,
    },
    CloseSession {
        session_id: String,
    },
    RunCommand {
        session_id: String,
        command: String,
        actor_name: Option<String>,
    },
    TailLog {
        session_id: String,
        path: String,
        lines: Option<u16>,
    },
    GetTerminalSnapshot {
        session_id: String,
        lines: Option<u16>,
    },
    GetPolicy,
    UpdatePolicy {
        mode: Option<String>,
        whitelist: Option<Vec<String>>,
        blacklist: Option<Vec<String>>,
    },
    FileList {
        session_id: String,
        remote_path: String,
        actor_name: Option<String>,
    },
    FileStat {
        session_id: String,
        remote_path: String,
        actor_name: Option<String>,
    },
    FileRead {
        session_id: String,
        remote_path: String,
        actor_name: Option<String>,
    },
    FileWrite {
        session_id: String,
        remote_path: String,
        content: String,
        actor_name: Option<String>,
    },
    FileDelete {
        session_id: String,
        remote_path: String,
        actor_name: Option<String>,
    },
    FileUpload {
        session_id: String,
        local_path: String,
        remote_path: String,
        actor_name: Option<String>,
    },
    FileDownload {
        session_id: String,
        remote_path: String,
        local_path: String,
        actor_name: Option<String>,
    },
}

pub fn tail_log_command(path: &str, lines: Option<u16>) -> String {
    let lines = lines.unwrap_or(100).clamp(1, 1000);
    format!("tail -n {lines} {}", shell_quote(path))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn tool(name: &str, description: &str, endpoint: &str, input_schema: Value) -> ToolDescriptor {
    ToolDescriptor {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema,
        gateway_endpoint: endpoint.to_owned(),
    }
}

fn file_tool(name: &str, description: &str, operation: &str) -> ToolDescriptor {
    tool(
        name,
        description,
        "/api/v1/files",
        json!({
            "type": "object",
            "required": ["session_id", "remote_path"],
            "properties": {
                "session_id": { "type": "string" },
                "remote_path": { "type": "string" },
                "actor_name": { "type": "string" },
                "operation": { "type": "string", "const": operation }
            },
            "additionalProperties": false
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_core_tools() {
        let names = default_tools()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"list_hosts".to_owned()));
        assert!(names.contains(&"run_command".to_owned()));
        assert!(names.contains(&"tail_log".to_owned()));
    }

    #[test]
    fn builds_bounded_tail_command() {
        assert_eq!(
            tail_log_command("/var/log/nginx/error.log", Some(2000)),
            "tail -n 1000 '/var/log/nginx/error.log'"
        );
    }
}
