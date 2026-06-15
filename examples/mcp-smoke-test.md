# Aegis MCP Smoke Test

This smoke test verifies the minimum Agent path:

```text
Codex -> MCP -> Aegis Gateway -> policy / approval / audit
```

It does not require a production server. For the first pass, the built-in
`demo-nginx` host is enough to validate MCP, sessions, policy, and audit.

## 0. Automated Gateway Preflight

Before the manual MCP calls, you can run the local Gateway smoke script in a
second terminal after Gateway starts:

```bash
npm run gateway:smoke
```

The script verifies `/health`, host sync, guarded policy, command
classification, low-risk command queueing, high-risk approval creation, and
JSONL audit export.

## 1. Start Gateway

From the repository root:

```bash
AEGIS_GATEWAY_DB=./temp/aegis-smoke.sqlite npm run gateway
```

Expected health check:

```bash
curl http://127.0.0.1:17321/health
```

Expected response:

```json
{
  "service": "aegis-gateway",
  "status": "ok"
}
```

## 2. Register MCP Server In Codex

```bash
codex mcp add aegis --url http://127.0.0.1:17321/mcp
```

For a persistent config example, see:

```text
examples/codex-config.toml
```

## 3. MCP Initialize

Send a JSON-RPC initialize request:

```bash
curl -i http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

Expected:

- HTTP 200
- `mcp-session-id` response header
- `serverInfo.name = aegis-gateway`

## 4. List Tools

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

Expected tool names include:

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
file_read
file_write
file_upload
file_download
```

## 5. List Hosts

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_hosts","arguments":{}}}'
```

Expected:

```text
demo-nginx
```

When the desktop app is running, this list should reflect electerm SSH/FTP
bookmarks synced by the Aegis Agent panel.

## 6. Open Session

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"open_session","arguments":{"host_id":"demo-nginx","title":"Smoke Session"}}}'
```

Expected:

- A session id is returned.
- `host_id` is `demo-nginx`.

Save the returned `session_id` for the next steps.

## 7. Low-Risk Command

Replace `<session_id>`:

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"run_command","arguments":{"session_id":"<session_id>","command":"pwd","actor_name":"Codex"}}}'
```

Expected without desktop executor:

```text
status = queued
terminal_command_id = <uuid>
output = queued: waiting for desktop terminal executor
```

Expected with desktop executor and a matching connected tab:

```text
status = executed
output = terminal output summary
```

## 8. High-Risk Command Approval

Replace `<session_id>`:

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"run_command","arguments":{"session_id":"<session_id>","command":"systemctl restart nginx","actor_name":"Codex"}}}'
```

Expected:

```text
status = approval_pending
approval_required = true
approval_id = <uuid>
```

The approval should appear in the Aegis Agent panel.

## 9. Audit Export

```bash
curl http://127.0.0.1:17321/api/v1/audit/events/export
```

Expected:

- Content type: `application/x-ndjson`
- One audit event per line
- Events include low-risk command queueing and high-risk approval request

## 10. Persistence Check

Stop Gateway, then start it again with the same database path:

```bash
AEGIS_GATEWAY_DB=./temp/aegis-smoke.sqlite npm run gateway
```

Check:

```bash
curl http://127.0.0.1:17321/api/v1/sessions
curl http://127.0.0.1:17321/api/v1/approvals
curl http://127.0.0.1:17321/api/v1/audit/events
```

Expected:

- The opened session is still present.
- Pending approvals are still present.
- Audit events are still present.

## Pass Criteria

The smoke test passes when:

- Gateway starts and responds to `/health`.
- MCP `tools/list` exposes Aegis tools.
- MCP `list_hosts` returns at least `demo-nginx`.
- MCP `open_session` returns a session id.
- Low-risk command creates a terminal command task or executes through desktop.
- High-risk command returns `approval_pending`.
- Audit JSONL export returns the operation timeline.
- Restarting Gateway with the same SQLite path restores session, approval, and
  audit state.
