# Aegis 架构说明

Aegis 是基于 electerm 二次开发的安全远程终端工作台。第一阶段保留 electerm 的 SSH、SFTP、多标签终端能力，在旁路增加 Rust Gateway，为 AI Coding Agent 提供受控的 MCP 工具入口。

## 模块边界

- `src/`：继承 electerm 桌面端代码，负责终端、SFTP、主机管理和 Electron UI。
- `crates/aegis-gateway`：本地 Rust 服务，承载 HTTP API、后续 WebSocket 事件和 MCP 路由入口。
- `crates/aegis-policy`：命令风险分级、只读模式、黑白名单和审批判定。
- `crates/aegis-audit`：审计事件模型、操作链路记录和后续回放数据结构。
- `crates/aegis-mcp`：面向 Codex 等 Agent 的 MCP tool 描述和工具路由适配层。
- `src/app/widgets/widget-aegis-gateway.js`：桌面端内置 Gateway Widget，负责从 electerm 启停和监控本地 Rust Gateway。
- `src/app/lib/aegis-gateway-client.js`：桌面端访问 Gateway HTTP API 的轻量 client。

## MVP 顺序

1. 跑通 electerm fork，确认桌面端可启动。
2. 跑通 `aegis-gateway`，桌面端可以显示 Gateway 健康状态。
3. 暴露最小 MCP tools：`list_hosts`、`run_command`、`tail_log`。
4. 接入 `aegis-policy`，高危命令进入审批队列。
5. 接入 `aegis-audit`，生成 session timeline 和 JSONL 导出。

## 当前骨架

当前提交只提供 Rust 侧最小可运行框架：

- `GET /health`
- `GET /api/v1/hosts`
- `POST /api/v1/sessions`
- `POST /api/v1/commands`
- `GET /api/v1/approvals`
- `POST /api/v1/approvals/{approval_id}/decision`
- `POST /api/v1/sessions/{session_id}/pause`
- `POST /api/v1/sessions/{session_id}/resume`
- `POST /api/v1/sessions/{session_id}/mode`
- `POST /api/v1/policy/classify`
- `GET /api/v1/audit/events`

后续桌面端应通过本地 HTTP/WebSocket 连接 Gateway，而不是让 Agent 直接持有 SSH 凭据。

## 桌面端接入策略

第一步使用 electerm 现有 Widgets 面板承载 `Aegis Gateway` 控制入口，避免一开始侵入主终端布局。该 Widget 当前负责：

- 启动/停止 `aegis-gateway`。
- 轮询 `/health` 判断 Gateway 状态。
- 提供 `classify`、`approvals`、`auditEvents` 调试函数。

后续独立 Agent Panel 可以复用同一个 `aegis-gateway-client`，再增加审批弹窗、会话 timeline 和人类暂停/接管按钮。
