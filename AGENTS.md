请使用简体中文回答。

本仓库是 Aegis，基于 electerm 二次开发，目标是为 Codex、Claude Code、Gemini CLI 等 AI Coding Agent 提供安全可控的远程终端工作台。

开发原则：
- 保留 electerm 的 SSH、SFTP、多标签终端等基础能力，优先以增量方式接入 Aegis 功能。
- Rust 侧能力放在 `crates/`，桌面端能力沿用 `src/` 内 electerm 现有结构。
- 高危终端操作必须经过策略引擎、审批和审计链路，不要绕过 Gateway 直接执行。
- 新增代码保持小步可验证，优先补齐测试或最小运行检查。
