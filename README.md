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
GET  /api/v1/sessions
POST /api/v1/sessions
POST /api/v1/sessions/{session_id}/pause
POST /api/v1/sessions/{session_id}/resume
POST /api/v1/sessions/{session_id}/mode
POST /api/v1/commands
GET  /api/v1/approvals
POST /api/v1/approvals/{approval_id}/decision
POST /api/v1/policy/classify
GET  /api/v1/audit/events
POST /mcp
```

## MCP 接入

Gateway 现在提供最小 MCP JSON-RPC HTTP 入口：

```text
http://127.0.0.1:17321/mcp
```

已暴露 tools：

```text
list_hosts
open_session
run_command
tail_log
get_terminal_snapshot
```

Codex 配置示例：

```toml
[mcp_servers.aegis]
url = "http://127.0.0.1:17321/mcp"
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

切到 `Aegis` 后，面板会先把 electerm 中的 SSH bookmarks 同步到 Gateway，再刷新 Gateway 状态。因此 MCP 的 `list_hosts` 返回的是桌面端当前 SSH 书签，而不是固定 demo host。

## MVP 路线

1. 跑通 electerm fork，确认桌面端可启动。
2. 前端连接 `aegis-gateway`，Agent Panel 显示 Gateway 状态。
3. 实现 MCP tools：`list_hosts`、`run_command`、`tail_log`。
4. 接入命令风险分级，高危命令触发审批。
5. 落地审计 timeline、JSONL 导出和操作回放。

## 上游来源

Aegis 基于 [electerm](https://github.com/electerm/electerm) fork 开发。当前本地 Git remote 使用 `upstream` 指向 electerm，后续应新增自己的 `origin`：

```bash
git remote add origin <your-aegis-repo-url>
git push -u origin aegis/main
```
