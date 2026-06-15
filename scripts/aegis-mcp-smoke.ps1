param(
  [string] $GatewayUrl = "http://127.0.0.1:17321",
  [string] $HostId = "demo-nginx"
)

$ErrorActionPreference = "Stop"
$script:McpSessionId = ""

function Join-AegisUrl {
  param([string] $Path)
  return $GatewayUrl.TrimEnd("/") + $Path
}

function Get-AegisResponseText {
  param([object] $Content)
  if ($Content -is [byte[]]) {
    return [System.Text.Encoding]::UTF8.GetString($Content)
  }
  return [string]$Content
}

function Invoke-AegisJson {
  param(
    [string] $Method,
    [string] $Path,
    [object] $Body = $null
  )

  $args = @{
    Method = $Method
    Uri = Join-AegisUrl $Path
  }

  if ($null -ne $Body) {
    $args.ContentType = "application/json"
    $args.Body = ($Body | ConvertTo-Json -Depth 24)
  }

  return Invoke-RestMethod @args
}

function Invoke-AegisMcpRaw {
  param(
    [object] $Body
  )

  $headers = @{}
  if (-not [string]::IsNullOrWhiteSpace($script:McpSessionId)) {
    $headers["mcp-session-id"] = $script:McpSessionId
  }

  $response = Invoke-WebRequest `
    -UseBasicParsing `
    -Method POST `
    -Uri (Join-AegisUrl "/mcp") `
    -ContentType "application/json" `
    -Headers $headers `
    -Body ($Body | ConvertTo-Json -Depth 32)

  if ($response.Headers["mcp-session-id"]) {
    $script:McpSessionId = [string]$response.Headers["mcp-session-id"]
  }

  $text = Get-AegisResponseText $response.Content
  return $text | ConvertFrom-Json
}

function Invoke-AegisMcp {
  param(
    [int] $Id,
    [string] $Method,
    [object] $Params = $null
  )

  $body = @{
    jsonrpc = "2.0"
    id = $Id
    method = $Method
  }
  if ($null -ne $Params) {
    $body.params = $Params
  }

  $response = Invoke-AegisMcpRaw $body
  if ($response.error) {
    throw "MCP $Method failed: $($response.error.message)"
  }
  return $response.result
}

function Invoke-AegisTool {
  param(
    [int] $Id,
    [string] $Name,
    [object] $Arguments = @{}
  )

  $result = Invoke-AegisMcp $Id "tools/call" @{
    name = $Name
    arguments = $Arguments
  }
  $text = [string]$result.content[0].text
  return $text | ConvertFrom-Json
}

function Assert-Aegis {
  param(
    [bool] $Condition,
    [string] $Message
  )

  if (-not $Condition) {
    throw "MCP smoke check failed: $Message"
  }

  Write-Host "[ok] $Message" -ForegroundColor Green
}

Write-Host "Aegis MCP smoke test: $GatewayUrl/mcp" -ForegroundColor Cyan

$health = Invoke-AegisJson "GET" "/health"
Assert-Aegis ($health.status -eq "ok") "gateway health is ok"

$null = Invoke-AegisJson "POST" "/api/v1/hosts/sync" @{
  hosts = @(
    @{
      id = $HostId
      name = "Demo nginx"
      address = "127.0.0.1:22"
      tags = @("demo", "ssh", "mcp-smoke")
    }
  )
}

$initialize = Invoke-AegisMcp 1 "initialize" @{}
Assert-Aegis ($initialize.serverInfo.name -eq "aegis-gateway") "mcp initialize returns aegis-gateway"
Assert-Aegis ([string]::IsNullOrWhiteSpace($script:McpSessionId) -eq $false) "mcp session id is returned"

$toolsResult = Invoke-AegisMcp 2 "tools/list" @{}
$toolNames = @($toolsResult.tools | ForEach-Object { $_.name })
foreach ($requiredTool in @("list_hosts", "open_session", "run_command", "get_policy", "update_policy", "file_upload", "file_download")) {
  Assert-Aegis ($toolNames -contains $requiredTool) "tool is exposed: $requiredTool"
}

$policy = Invoke-AegisTool 3 "update_policy" @{
  mode = "guarded"
  whitelist = @()
  blacklist = @()
}
Assert-Aegis ($policy.mode -eq "guarded") "mcp update_policy switches to guarded"

$hosts = @(Invoke-AegisTool 4 "list_hosts" @{})
Assert-Aegis (@($hosts | Where-Object { $_.id -eq $HostId }).Count -eq 1) "mcp list_hosts sees synced demo host"

$session = Invoke-AegisTool 5 "open_session" @{
  host_id = $HostId
  title = "Aegis MCP Smoke Session"
}
Assert-Aegis ([string]::IsNullOrWhiteSpace($session.id) -eq $false) "mcp open_session returns a session"

$lowCommand = Invoke-AegisTool 6 "run_command" @{
  session_id = $session.id
  command = "pwd"
  actor_name = "Codex MCP Smoke"
}
Assert-Aegis (@("queued", "executed") -contains $lowCommand.status) "mcp run_command queues low-risk command"
Assert-Aegis ($lowCommand.risk -eq "low") "mcp low-risk command reports low risk"

$highCommand = Invoke-AegisTool 7 "run_command" @{
  session_id = $session.id
  command = "systemctl restart nginx"
  actor_name = "Codex MCP Smoke"
}
Assert-Aegis ($highCommand.status -eq "approval_pending") "mcp run_command requests approval for high-risk command"
Assert-Aegis ($highCommand.approval_required -eq $true) "mcp high-risk command marks approval_required"
Assert-Aegis ([string]::IsNullOrWhiteSpace($highCommand.approval_id) -eq $false) "mcp high-risk command returns approval id"

$snapshot = Invoke-AegisTool 8 "get_terminal_snapshot" @{
  session_id = $session.id
}
Assert-Aegis ($snapshot.session_id -eq $session.id) "mcp get_terminal_snapshot accepts the session"

$closed = Invoke-AegisTool 9 "close_session" @{
  session_id = $session.id
}
Assert-Aegis ($closed.id -eq $session.id) "mcp close_session closes the smoke session"

Write-Host ""
Write-Host "MCP smoke test passed." -ForegroundColor Green
Write-Host "Session:  $($session.id)"
Write-Host "Approval: $($highCommand.approval_id)"
