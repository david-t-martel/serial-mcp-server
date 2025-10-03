Param(
  [int]$Port = 4226,
  [string]$CacheDir = "$env:LOCALAPPDATA/sccache/cache",
  [string]$MaxSize = "15G"
)

$ConfigPath = Join-Path $env:APPDATA "Mozilla/sccache/config"
New-Item -ItemType Directory -Force -Path $ConfigPath | Out-Null
$file = Join-Path $ConfigPath "config"

@"
[cache.disk]
dir = "$CacheDir"
size = "$MaxSize"

[server]
port = $Port
"@ | Set-Content -Encoding UTF8 $file

Write-Host "sccache config written to $file"
