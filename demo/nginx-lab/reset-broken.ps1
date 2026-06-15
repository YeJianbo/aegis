$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$config = Join-Path $root 'nginx/conf.d/default.conf'
$fixed = Join-Path $root 'nginx/conf.d/default.conf.fixed'

$content = Get-Content -LiteralPath $fixed -Raw
$broken = $content.Replace('root /usr/share/nginx/html;', 'root /usr/share/nginx/html')
Set-Content -LiteralPath $config -Value $broken -NoNewline

Push-Location $root
try {
  docker compose down
} finally {
  Pop-Location
}
