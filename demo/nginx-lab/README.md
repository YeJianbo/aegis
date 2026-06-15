# Aegis Nginx Failure Lab

This demo creates a small nginx failure that is safe to run locally. It is
designed to show the Aegis story:

```text
Agent investigates -> reads config/logs -> proposes a fix -> uploads patch
-> requests restart -> human approval -> audit export
```

The lab starts with a broken nginx config. The missing semicolon after `root`
causes `nginx -t` and the container start to fail.

## Prerequisites

- Docker or Docker Desktop
- Aegis Gateway running on `127.0.0.1:17321`
- Optional: Aegis desktop connected to a local/VM SSH bookmark that can run
  Docker commands

## 1. Start The Broken Lab

From this directory:

```bash
docker compose up -d
docker compose ps
docker logs aegis-nginx-lab
```

Expected:

```text
nginx: [emerg] invalid number of arguments in "root" directive
```

The nginx container should exit because `nginx/conf.d/default.conf` is broken.

## 2. Suggested Agent Prompt

Use this prompt in Codex or another MCP-capable coding agent:

```text
Use Aegis to investigate why the nginx lab is not starting.
Read the nginx config, identify the syntax issue, patch it, validate with
nginx -t or docker logs, then request a safe restart through Aegis.
Do not run destructive commands.
```

## 3. Expected Agent Commands

Low/medium-risk investigation:

```bash
pwd
ls -la demo/nginx-lab
docker compose -f demo/nginx-lab/docker-compose.yml ps
docker logs aegis-nginx-lab
cat demo/nginx-lab/nginx/conf.d/default.conf
```

Patch:

```bash
cp demo/nginx-lab/nginx/conf.d/default.conf.fixed demo/nginx-lab/nginx/conf.d/default.conf
```

High-risk approval point:

```bash
cd demo/nginx-lab && docker compose restart nginx
```

Aegis should classify the restart as high risk and create an approval request.
The human operator reviews it in the Aegis Agent panel, then clicks
`Allow once` or `Whitelist & Allow`.

## 4. Manual MCP Flow

If you want to test without a full Codex UI, use the Gateway MCP endpoint
directly. First open a session:

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"open_session","arguments":{"host_id":"demo-nginx","title":"Nginx Lab"}}}'
```

Save the returned `session_id`.

Run an investigation command:

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_command","arguments":{"session_id":"<session_id>","command":"docker logs aegis-nginx-lab","actor_name":"Codex"}}}'
```

Request the high-risk restart:

```bash
curl http://127.0.0.1:17321/mcp \
  -H "content-type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_command","arguments":{"session_id":"<session_id>","command":"cd demo/nginx-lab && docker compose restart nginx","actor_name":"Codex"}}}'
```

Expected:

```text
status = approval_pending
approval_required = true
approval_id = <uuid>
```

## 5. Verify The Fix

After approval and restart:

```bash
docker compose ps
curl http://127.0.0.1:18080
```

Expected page text:

```text
Aegis Nginx Lab
```

## 6. Export Audit Timeline

```bash
curl http://127.0.0.1:17321/api/v1/audit/events/export > aegis-nginx-lab-audit.jsonl
```

The JSONL file should show:

- investigation commands
- config read/write or copy action
- restart approval request
- human approval decision
- final execution result

## 7. Reset The Lab

To restore the broken config:

```bash
sh reset-broken.sh
```

On Windows PowerShell:

```powershell
.\reset-broken.ps1
```

The next `docker compose up -d` will reproduce the failure.

## Demo Talking Points

- Aegis keeps the remote shell visible in electerm.
- Low-risk commands can run through the terminal executor.
- High-risk restart creates a human approval request.
- The user can deny, modify, allow once, or whitelist the command.
- Every step is written into the audit timeline and exportable as JSONL.
