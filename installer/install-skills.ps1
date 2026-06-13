$ErrorActionPreference = "Stop"

$Repo = "engrammic-ai/mcp"
$Binary = "engrammic"

Write-Host ""
Write-Host "Engrammic Skills Installer"
Write-Host ""

# Windows is always x86_64-pc-windows-msvc for now
$Target = "x86_64-pc-windows-msvc"
Write-Host "Detected: $Target"

$ReleaseUrl = "https://github.com/$Repo/releases/latest/download/$Binary-$Target.exe"

# Download to temp
$TempDir = $env:TEMP
$Installer = Join-Path $TempDir "$Binary.exe"

Write-Host "=> Downloading skills installer..."
Invoke-WebRequest -Uri $ReleaseUrl -OutFile $Installer -UseBasicParsing

Write-Host "=> Installing skills..."

# Run skills-only install with auto-accept
& $Installer skills -y @args

Write-Host ""
Write-Host "=> Skills installation complete!" -ForegroundColor Green
