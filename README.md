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
POST /api/v1/policy/classify
GET  /api/v1/audit/events
POST /api/v1/audit/events
```

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
