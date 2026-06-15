param(
  [string] $GatewayUrl = "http://127.0.0.1:17321",
  [string] $HostId = "demo-nginx",
  [string] $ExportPath = ""
)

$ErrorActionPreference = "Stop"

function Join-AegisUrl {
  param([string] $Path)
  return $GatewayUrl.TrimEnd("/") + $Path
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
    $args.Body = ($Body | ConvertTo-Json -Depth 16)
  }

  return Invoke-RestMethod @args
}

function Assert-Aegis {
  param(
    [bool] $Condition,
    [string] $Message
  )

  if (-not $Condition) {
    throw "Smoke check failed: $Message"
  }

  Write-Host "[ok] $Message" -ForegroundColor Green
}

function Get-ArrayCount {
  param([object] $Value)
  if ($null -eq $Value) {
    return 0
  }
  return @($Value).Count
}

function Get-AegisResponseText {
  param([object] $Content)
  if ($Content -is [byte[]]) {
    return [System.Text.Encoding]::UTF8.GetString($Content)
  }
  return [string]$Content
}

if (-not $ExportPath) {
  $tempDir = Join-Path (Get-Location) "temp"
  New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
  $ExportPath = Join-Path $tempDir "aegis-smoke-audit.jsonl"
}

Write-Host "Aegis Gateway smoke test: $GatewayUrl" -ForegroundColor Cyan

$health = Invoke-AegisJson "GET" "/health"
Assert-Aegis ($health.status -eq "ok") "gateway health is ok"

$hosts = Invoke-AegisJson "POST" "/api/v1/hosts/sync" @{
  hosts = @(
    @{
      id = $HostId
      name = "Demo nginx"
      address = "127.0.0.1:22"
      tags = @("demo", "ssh", "smoke")
    }
  )
}
Assert-Aegis (@($hosts | Where-Object { $_.id -eq $HostId }).Count -eq 1) "demo host is synced"

$policy = Invoke-AegisJson "PUT" "/api/v1/policy" @{
  mode = "guarded"
  whitelist = @()
  blacklist = @()
}
Assert-Aegis ($policy.mode -eq "guarded") "policy is guarded"

$low = Invoke-AegisJson "POST" "/api/v1/policy/classify" @{
  command = "pwd"
}
Assert-Aegis ($low.risk -eq "low") "low-risk command classification works"

$high = Invoke-AegisJson "POST" "/api/v1/policy/classify" @{
  command = "systemctl restart nginx"
}
Assert-Aegis (@("high", "critical") -contains $high.risk) "high-risk command classification works"

$session = Invoke-AegisJson "POST" "/api/v1/sessions" @{
  host_id = $HostId
  title = "Aegis Smoke Session"
}
Assert-Aegis ([string]::IsNullOrWhiteSpace($session.id) -eq $false) "session opens"

$lowCommand = Invoke-AegisJson "POST" "/api/v1/commands" @{
  session_id = $session.id
  command = "pwd"
  actor_name = "Codex Smoke"
}
Assert-Aegis (@("queued", "executed") -contains $lowCommand.status) "low-risk command is queued or executed"

$highCommand = Invoke-AegisJson "POST" "/api/v1/commands" @{
  session_id = $session.id
  command = "systemctl restart nginx"
  actor_name = "Codex Smoke"
}
Assert-Aegis ($highCommand.status -eq "approval_pending") "high-risk command requests approval"
Assert-Aegis ([string]::IsNullOrWhiteSpace($highCommand.approval_id) -eq $false) "approval id is returned"

$approvals = Invoke-AegisJson "GET" "/api/v1/approvals"
Assert-Aegis (@($approvals | Where-Object { $_.id -eq $highCommand.approval_id }).Count -eq 1) "approval is visible"

$auditEvents = Invoke-AegisJson "GET" "/api/v1/audit/events"
Assert-Aegis ((Get-ArrayCount $auditEvents) -ge 2) "audit events are recorded"

$export = Invoke-WebRequest -UseBasicParsing -Method GET -Uri (Join-AegisUrl "/api/v1/audit/events/export")
$exportText = Get-AegisResponseText $export.Content
Assert-Aegis ($export.Headers["Content-Type"] -like "application/x-ndjson*") "audit export uses jsonl content type"
Assert-Aegis ($exportText -match "systemctl restart nginx") "audit export contains the high-risk command"

Set-Content -Path $ExportPath -Value $exportText -Encoding UTF8
Write-Host ""
Write-Host "Smoke test passed." -ForegroundColor Green
Write-Host "Session:  $($session.id)"
Write-Host "Approval: $($highCommand.approval_id)"
Write-Host "Export:   $ExportPath"
