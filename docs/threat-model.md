# Aegis Threat Model

This document defines the security assumptions for the Aegis MVP. Aegis does
not try to make remote shell access risk-free. It reduces the blast radius of
AI-driven terminal operations by making them visible, policy-gated,
interruptible, and auditable.

## Assets

Protected assets:

- SSH private keys and saved credentials managed by the electerm desktop app.
- Remote server shells, files, processes, services, and deployment state.
- Human approval decisions and audit records.
- Local gateway policy configuration.
- Agent-visible command output, which may contain secrets or operational data.

## Actors

```text
Human operator
  owns the desktop session, SSH credentials, and approval decisions

AI Coding Agent
  calls MCP tools but should not directly own SSH credentials or raw shell
  access

Aegis Gateway
  local policy, approval, queue, MCP, and audit broker

electerm Desktop
  visible terminal and SFTP/FTP executor

Remote server
  target system reached through electerm SSH/SFTP/FTP connections
```

## Trust Boundaries

```text
Agent process
  untrusted for shell safety; may hallucinate, overreach, or be prompt-injected

Local Gateway
  trusted policy decision point, but it only controls requests routed through
  Aegis

Desktop terminal
  trusted execution surface visible to the human operator

Remote host
  outside local trust; command output is treated as untrusted input
```

The most important boundary is between the AI Agent and the remote shell. The
agent can request actions through MCP, but the Gateway and desktop decide what
is allowed and how it is executed.

## In-Scope Threats

### Unsafe Agent Command Execution

Risk: the agent asks to run destructive commands such as `rm -rf`, `chmod -R`,
`systemctl restart`, `docker restart`, `iptables`, `kubectl delete`, or
database mutations.

Mitigations:

- Static command risk classification.
- Guarded mode with approval queue for high and critical commands.
- Blacklist rules that hard-block known destructive patterns.
- Human Allow / Deny / Modify workflow before execution.
- Visible terminal injection instead of hidden background shell execution.

### Prompt Injection Through Logs Or Files

Risk: remote logs or files contain instructions that attempt to make the agent
ignore policy and run unsafe commands.

Mitigations:

- Agent policy is enforced by Gateway, not by the model prompt alone.
- High-risk commands still require approval.
- Audit timeline records the command chain that followed suspicious output.

### Credential Exfiltration

Risk: the agent tries to read local SSH keys or secret files, or remote command
output leaks secrets.

Mitigations:

- Agent does not receive SSH private keys.
- Agent uses host/session identifiers instead of raw credentials.
- File tools are routed through existing desktop sessions and policy modes.
- Future work: structured redaction rules for common secret formats.

### Human Loses Control Of Terminal Session

Risk: the agent keeps issuing commands while the human is trying to inspect or
recover the remote environment.

Mitigations:

- Pause Agent at session level.
- `human_only` mode blocks agent commands and file tasks.
- Session close removes the Gateway-controlled session binding.
- Approval queue blocks high-risk commands until human action.

### Audit Gaps

Risk: executed commands are not attributable to the agent or cannot be replayed
after an incident.

Mitigations:

- Gateway records actor, host, session, command, risk, approval status, exit
  code, output summary, and timestamps.
- Approval decisions are recorded as separate audit events.
- File operations are audited alongside terminal commands.

## Out Of Scope For MVP

- Preventing malicious behavior by a user who already controls the desktop.
- Protecting against a fully compromised local machine.
- Verifying all remote command semantics with shell-perfect parsing.
- Replacing SSH server-side authorization, sudo policy, or OS-level controls.
- Full secret redaction for every possible custom credential format.

## Default Security Posture

The recommended MVP default is:

```text
policy.mode = guarded
agent session mode = agent_writable only for explicit active sessions
high / critical commands = approval required
blacklist = enabled
audit = always on
gateway bind address = 127.0.0.1
```

`full_allow` is intended for trusted local demos or tightly scoped lab
environments. Blacklist rules still apply in full-allow mode.

## Demo Safety Rules

For public demos and interviews:

- Use a disposable VM or container as the remote host.
- Prefer harmless commands for the normal path: `pwd`, `ls`, `git status`,
  `tail`, `cat`, `nginx -t`.
- Use `systemctl restart nginx` or an equivalent lab-only restart to show
  approval.
- Avoid running destructive examples such as `rm -rf` against a real server.
