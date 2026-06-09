# Aegis Architecture

Aegis is a human-in-the-loop remote terminal workspace for AI Coding Agents.
It keeps electerm as the desktop SSH/SFTP surface and adds a local Rust
Gateway that exposes controlled MCP tools to Codex, Claude Code, Gemini CLI,
and similar agents.

The core architectural rule is simple: agents never own raw SSH credentials
and never execute an unrestricted shell directly. They call Aegis tools, Aegis
classifies the request, the desktop user can approve or take over, and every
operation is audited.

## System Map

```text
Codex / Claude Code / Gemini CLI
            |
            | MCP Streamable HTTP
            v
      Aegis Rust Gateway
            |
            | HTTP queue / WebSocket events
            v
      electerm-based Desktop
            |
            | SSH PTY / SFTP / FTP
            v
        Remote Server
```

## Repository Layout

```text
src/
  electerm desktop application, terminal UI, SFTP/FTP UI, widgets, Agent panel

src/app/widgets/widget-aegis-gateway.js
  desktop widget that starts and monitors the local Rust Gateway

src/app/lib/aegis-gateway-client.js
  desktop-side HTTP client used by the widget, Agent panel, terminal executor,
  and file executor

crates/aegis-gateway
  Axum/Tokio service, HTTP API, MCP endpoint, queues, approvals, shared state

crates/aegis-policy
  command risk classifier, guarded/full-allow modes, whitelist, blacklist

crates/aegis-audit
  audit event model for command/file/session/approval timeline records

crates/aegis-mcp
  MCP tool schema definitions and agent-facing tool descriptions
```

## Command Flow

```text
Agent calls run_command
        |
        v
Gateway validates session and policy
        |
        +-- blocked: return policy_denied and audit
        |
        +-- approval required: create approval request and wait for user
        |
        +-- allowed: enqueue terminal command
                    |
                    v
Desktop terminal executor finds the matching live SSH tab
                    |
                    v
Command is injected into the real electerm terminal
                    |
                    v
Output is captured after terminal idle
                    |
                    v
Gateway records audit event and returns result to Agent
```

The command executor intentionally uses the visible electerm terminal instead
of a hidden shell. This is the main user-facing property of Aegis: the human
operator can see, pause, deny, modify, or take over the same terminal session
that the agent is using.

## File Flow

```text
Agent calls file_read / file_write / file_upload / file_download
        |
        v
Gateway validates session mode and policy
        |
        v
Gateway enqueues a file task
        |
        v
Desktop file executor maps the task to an existing SSH/SFTP/FTP tab
        |
        v
electerm SFTP/FTP APIs perform the operation
        |
        v
Gateway records the file audit event
```

Write, delete, upload, and download operations are blocked when the session is
paused, in `human_only` mode, or in `agent_read_only` mode where applicable.

## Policy Model

Aegis policy has three layers:

```text
mode=guarded
  normal mode; risk classification controls approval behavior

mode=full_allow
  skips normal approval, but blacklist rules still hard-block commands

whitelist / blacklist
  explicit pattern-level overrides for local project or server rules
```

The current static classifier separates shell commands into low, medium, high,
and critical risk. High and critical commands such as `systemctl restart`,
`docker restart`, `chmod -R`, `rm -rf`, `iptables`, and destructive database or
Kubernetes operations are routed to the approval queue.

## Session Model

Each Aegis session is bound to a desktop-visible host and can be placed in one
of these modes:

```text
agent_writable
  agent can run allowed commands and allowed file tasks

agent_read_only
  agent can inspect state but cannot write files or perform mutating operations

human_only
  agent requests are blocked until the user restores agent access
```

Sessions can also be paused. Pausing blocks new agent work while preserving the
terminal tab and audit context.

## Gateway API Surface

The Gateway listens on `127.0.0.1:17321` by default.

```text
GET    /health
GET    /api/v1/hosts
POST   /api/v1/hosts/sync
GET    /api/v1/sessions
POST   /api/v1/sessions
DELETE /api/v1/sessions/{session_id}
POST   /api/v1/sessions/{session_id}/pause
POST   /api/v1/sessions/{session_id}/resume
POST   /api/v1/sessions/{session_id}/mode
POST   /api/v1/commands
POST   /api/v1/terminal/commands/next
POST   /api/v1/terminal/commands/{command_id}/complete
GET    /api/v1/approvals
POST   /api/v1/approvals/{approval_id}/decision
GET    /api/v1/policy
PUT    /api/v1/policy
POST   /api/v1/policy/classify
GET    /api/v1/audit/events
GET    /mcp
POST   /mcp
DELETE /mcp
```

## MCP Tools

The MCP endpoint is:

```text
http://127.0.0.1:17321/mcp
```

Current tools:

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

## MVP Acceptance Chain

The project is considered demo-ready when this chain is stable:

```text
Codex -> MCP -> Aegis Gateway -> electerm terminal -> remote server
      -> approval/audit events -> Aegis Agent panel
```

The primary demo scenario is AI-assisted service troubleshooting: read logs,
inspect configuration, upload a patch, request service restart, trigger human
approval, execute the restart in the visible terminal, and replay the audit
timeline.
