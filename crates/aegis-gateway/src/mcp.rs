use aegis_mcp::{default_tools, tail_log_command};
use aegis_policy::RiskLevel;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::convert::Infallible;
use uuid::Uuid;

use crate::command_flow::submit_command;
use crate::models::{CommandStatus, HostSummary, RunCommandRequest, SessionSummary};
use crate::state::{AppState, new_session};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

const MCP_SESSION_ID_HEADER: &str = "mcp-session-id";

pub async fn mcp_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    let id = request.id.clone();
    let is_initialize = request.method == "initialize";
    if request.jsonrpc.as_deref().unwrap_or("2.0") != "2.0" {
        return json_rpc_response(error_response(id, -32600, "invalid jsonrpc version"), false);
    }

    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "aegis-gateway",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "notifications/initialized" => Ok(json!({})),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({
            "tools": default_tools()
                .into_iter()
                .map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema
                }))
                .collect::<Vec<_>>()
        })),
        "tools/call" => match parse_params::<ToolCallParams>(request.params) {
            Ok(params) => call_tool(state, params).await,
            Err(err) => Err(err),
        },
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("method not found: {}", request.method),
        }),
    };

    if id.is_none() {
        return StatusCode::ACCEPTED.into_response();
    }

    let response = match result {
        Ok(value) => success_response(id, value),
        Err(err) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(err),
        },
    };

    let mut response = json_rpc_response(response, is_initialize);
    if is_initialize {
        let session_id = headers
            .get(MCP_SESSION_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if let Ok(header_value) = HeaderValue::from_str(&session_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static(MCP_SESSION_ID_HEADER), header_value);
        }
    }
    response
}

pub async fn mcp_get() -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    Sse::new(stream::pending()).keep_alive(KeepAlive::default())
}

pub async fn mcp_delete() -> StatusCode {
    StatusCode::OK
}

async fn call_tool(state: AppState, params: ToolCallParams) -> Result<Value, JsonRpcError> {
    let payload = match params.name.as_str() {
        "list_hosts" => json!(list_hosts_for_mcp(&state).await),
        "open_session" => {
            let host_id = required_string(&params.arguments, "host_id")?;
            let title = optional_string(&params.arguments, "title");
            json!(open_session_for_mcp(&state, host_id, title).await?)
        }
        "run_command" => {
            let session_id = required_string(&params.arguments, "session_id")?;
            let command = required_string(&params.arguments, "command")?;
            let actor_name = optional_string(&params.arguments, "actor_name")
                .unwrap_or_else(|| "MCP Agent".to_owned());
            json!(run_command_for_mcp(&state, session_id, command, actor_name).await?)
        }
        "tail_log" => {
            let session_id = required_string(&params.arguments, "session_id")?;
            let path = required_string(&params.arguments, "path")?;
            let lines = params
                .arguments
                .get("lines")
                .and_then(Value::as_u64)
                .map(|value| value.min(u16::MAX as u64) as u16);
            let command = tail_log_command(&path, lines);
            json!(run_command_for_mcp(&state, session_id, command, "MCP Agent".to_owned()).await?)
        }
        "get_terminal_snapshot" => {
            let session_id = required_string(&params.arguments, "session_id")?;
            json!(terminal_snapshot_for_mcp(&state, session_id).await?)
        }
        _ => {
            return Err(JsonRpcError {
                code: -32602,
                message: format!("unknown tool: {}", params.name),
            });
        }
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_owned())
        }],
        "isError": false
    }))
}

async fn list_hosts_for_mcp(state: &AppState) -> Vec<HostSummary> {
    state.read().await.hosts.values().cloned().collect()
}

async fn open_session_for_mcp(
    state: &AppState,
    host_id: String,
    title: Option<String>,
) -> Result<SessionSummary, JsonRpcError> {
    let mut store = state.write().await;
    if !store.hosts.contains_key(&host_id) {
        return Err(invalid_params("host not found"));
    }
    let session = new_session(host_id, title);
    store.sessions.insert(session.id.clone(), session.clone());
    Ok(session)
}

#[derive(Debug, Serialize)]
struct McpCommandResult {
    status: CommandStatus,
    session_id: String,
    risk: RiskLevel,
    approval_required: bool,
    approval_id: Option<Uuid>,
    output: Option<String>,
    audit_event_id: Uuid,
    terminal_command_id: Option<Uuid>,
}

async fn run_command_for_mcp(
    state: &AppState,
    session_id: String,
    command: String,
    actor_name: String,
) -> Result<McpCommandResult, JsonRpcError> {
    let response = submit_command(
        state.clone(),
        RunCommandRequest {
            session_id,
            command,
            actor_name: Some(actor_name),
        },
    )
    .await
    .map_err(|err| JsonRpcError {
        code: -32602,
        message: err.message,
    })?;

    Ok(McpCommandResult {
        status: response.status,
        session_id: response.session_id,
        risk: response.assessment.risk,
        approval_required: response.assessment.approval_required,
        approval_id: response.approval_id,
        output: response.output,
        audit_event_id: response.audit_event.id,
        terminal_command_id: response.terminal_command_id,
    })
}

#[derive(Debug, Serialize)]
struct TerminalSnapshot {
    session_id: String,
    output: Vec<String>,
}

async fn terminal_snapshot_for_mcp(
    state: &AppState,
    session_id: String,
) -> Result<TerminalSnapshot, JsonRpcError> {
    let store = state.read().await;
    if !store.sessions.contains_key(&session_id) {
        return Err(invalid_params("session not found"));
    }
    let output = store
        .audit_events
        .iter()
        .filter(|event| event.session_id == session_id)
        .filter_map(|event| event.output_summary.clone())
        .collect();
    Ok(TerminalSnapshot { session_id, output })
}

fn parse_params<T: for<'de> Deserialize<'de>>(params: Option<Value>) -> Result<T, JsonRpcError> {
    serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|err| JsonRpcError {
        code: -32602,
        message: format!("invalid params: {err}"),
    })
}

fn required_string(value: &Value, key: &str) -> Result<String, JsonRpcError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_params(format!("missing required string: {key}")))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
}

fn success_response(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

fn invalid_params(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError {
        code: -32602,
        message: message.into(),
    }
}

fn json_rpc_response(response: JsonRpcResponse, is_initialize: bool) -> Response {
    let mut response = Json(response).into_response();
    if is_initialize {
        response.headers_mut().insert(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-cache"),
        );
    }
    response
}
