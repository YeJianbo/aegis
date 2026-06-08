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
