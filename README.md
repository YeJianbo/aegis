# Aegis

Aegis 是面向 AI Coding Agent 的安全远程终端工作台，基于 electerm 二次开发。它保留 SSH 终端、SFTP 文件管理、服务器分组、多标签会话等基础能力，并新增 Rust Agent Gateway，让 Codex、Claude Code、Gemini CLI 等 Agent 能通过受控 MCP 工具接入远程环境。

核心目标不是重新实现 SSH 客户端，而是把高危 Shell 执行能力封装为人类可见、可审批、可暂停、可接管、可审计的 Agent Tool。

## 当前状态

本仓库刚完成 fork 基线和 Aegis 框架搭建：

- `src/`：electerm 桌面端基础代码。
- `crates/aegis-gateway`：Rust Gateway 最小 HTTP 服务。
- `crates/aegis-policy`：命令风险分级骨架。
- `crates/aegis-audit`：审计事件模型骨架。
- `crates/aegis-mcp`：MCP tool 描述骨架。
- `docs/architecture.md`：Aegis Gateway、桌面端和 MCP 的系统链路说明。
- `docs/threat-model.md`：AI Agent 远程终端安全边界和威胁模型。
- `examples/codex-config.toml`：Codex 连接 Aegis MCP Gateway 的配置示例。
- `docs/electerm/README_UPSTREAM.md`：原 electerm README 归档。

## 快速验证

```bash
npm run gateway:test
npm run gateway
```

Gateway 默认监听：

```text
http://127.0.0.1:17321
```

已提供的最小接口：

```text
GET  /health
GET  /api/v1/hosts
POST /api/v1/hosts/sync
GET  /api/v1/sessions
POST /api/v1/sessions
DELETE /api/v1/sessions/{session_id}
POST /api/v1/sessions/{session_id}/pause
POST /api/v1/sessions/{session_id}/resume
POST /api/v1/sessions/{session_id}/mode
POST /api/v1/commands
POST /api/v1/files
POST /api/v1/files/tasks/next
POST /api/v1/files/tasks/{task_id}/complete
POST /api/v1/terminal/commands/next
POST /api/v1/terminal/commands/{command_id}/complete
GET  /api/v1/approvals
POST /api/v1/approvals/{approval_id}/decision
POST /api/v1/policy/classify
GET  /api/v1/policy
PUT  /api/v1/policy
GET  /api/v1/audit/events
GET/POST/DELETE /mcp
```

## MCP 接入

Gateway 现在提供 Codex 可连接的 Streamable HTTP MCP 入口：

```text
http://127.0.0.1:17321/mcp
```

已暴露 tools：

```text
list_hosts
open_session
close_session
run_command
tail_log
get_terminal_snapshot
get_policy
update_policy
file_list
file_stat
file_read
file_write
file_delete
file_upload
file_download
```

Codex 配置示例：

```bash
codex mcp add aegis --url http://127.0.0.1:17321/mcp
```

也可以参考仓库内配置文件：

```text
examples/codex-config.toml
```

建议把 Aegis 工具收敛到当前白名单，并让 Codex 外层默认放行；远程命令安全由 Aegis Gateway 的风险分级、审批队列和审计日志承担：

```toml
[mcp_servers.aegis]
url = "http://127.0.0.1:17321/mcp"
enabled_tools = [
  "list_hosts",
  "open_session",
  "close_session",
  "run_command",
  "tail_log",
  "get_terminal_snapshot",
  "get_policy",
  "update_policy",
  "file_list",
  "file_stat",
  "file_read",
  "file_write",
  "file_delete",
  "file_upload",
  "file_download",
]
default_tools_approval_mode = "approve"
```

MCP 调用流程：

```text
initialize
tools/list
tools/call list_hosts
tools/call open_session
tools/call run_command
```

高危命令不会直接执行，会返回：

```json
{
  "status": "approval_pending",
  "approval_required": true,
  "approval_id": "..."
}
```

策略工具支持三层控制：

```text
mode=guarded     使用风险分级和审批
mode=full_allow  默认不审批，但黑名单仍然硬阻断
whitelist        命中后跳过审批
blacklist        命中后直接阻断
```

## 桌面端接入

当前已新增内置 Widget：

```text
Aegis Gateway
```

在 electerm 的 Widgets 面板中启动该 Widget 后，它会在本地启动 Rust Gateway，并显示 Gateway URL。开发模式默认执行：

```bash
cargo run -p aegis-gateway
```

Widget 实例还暴露了用于调试的函数：

```text
status
classify
approvals
auditEvents
```

桌面端主入口已集成到 AI 面板：

```text
AI 面板 -> Chat / Aegis
```

切到 `Aegis` 后，面板会先把 electerm 中的 SSH/FTP bookmarks 同步到 Gateway，再刷新 Gateway 状态。因此 MCP 的 `list_hosts` 返回的是桌面端当前书签，而不是固定 demo host。

桌面端还会启动 Aegis terminal executor：

```text
Gateway command queue -> electerm active/matched SSH tab -> terminal input -> idle output capture -> Gateway audit
```

因此 `run_command` 不再是模拟输出。低风险命令会真实注入已连接或匹配书签的 SSH 终端；高危命令先进入审批队列，用户在 Aegis 面板中 Allow / Deny / Modify 后，批准的命令才会进入同一条真实终端执行链路。

桌面端同时启动 Aegis file executor：

```text
Gateway file task queue -> matched SSH/SFTP/FTP tab -> electerm SFTP API -> Gateway audit
```

文件工具走 electerm 已有 SFTP/FTP 能力，支持目录列表、状态读取、文本读取、文本写入、删除、本地上传和下载。写操作在 `agent_read_only` 下会被 Gateway 阻断，`human_only` 或 pause 状态下所有 Agent 文件操作都会被阻断。

## MVP 路线

1. 跑通 electerm fork，确认桌面端可启动。
2. 前端连接 `aegis-gateway`，Agent Panel 显示 Gateway 状态。
3. 实现 MCP tools：`list_hosts`、`run_command`、`tail_log`。
4. 接入命令风险分级，高危命令触发审批。
5. 落地审计 timeline、JSONL 导出和操作回放。

## 项目文档

```text
docs/architecture.md
docs/threat-model.md
examples/codex-config.toml
NOTICE
```

## 上游来源

Aegis 基于 [electerm](https://github.com/electerm/electerm) fork 开发。当前仓库 remote 约定为：

```text
origin   https://github.com/YeJianbo/aegis.git
upstream https://github.com/electerm/electerm.git
```

electerm 上游 README 已归档到 `docs/electerm/README_UPSTREAM.md`，版权和来源说明保留在 `LICENSE` 与 `NOTICE` 中。
