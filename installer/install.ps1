#Requires -Version 5.1
# install.ps1 — Engrammic installer for Windows
# Usage: Invoke-Expression (Invoke-WebRequest -Uri https://get.engrammic.ai/install.ps1 -UseBasicParsing).Content
#        ... | Invoke-Expression  (note: @args passthrough not possible via I-Ex; use iwr + &)
# For arg passthrough: & ([scriptblock]::Create((iwr https://get.engrammic.ai/install.ps1).Content)) -y --tool cursor
$ErrorActionPreference = "Stop"

$Repo        = "engrammic-ai/mcp"
$Binary      = "engrammic"
$Arch        = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "Arm64") { "aarch64" } else { "x86_64" }
$Target      = "$Arch-pc-windows-msvc"
$InstallDir  = Join-Path $env:LOCALAPPDATA "engrammic\bin"
$ReleaseBase = "https://github.com/$Repo/releases/latest/download"

Write-Host ""
Write-Host "Engrammic Setup" -ForegroundColor Cyan
Write-Host ""

Write-Host "=> Detected platform: $Target"

# ── Download binary + checksum ────────────────────────────────────────────────
$BinUrl = "$ReleaseBase/$Binary-$Target.exe"
$SumUrl = "$ReleaseBase/$Binary-$Target.exe.sha256"

$TmpBin = Join-Path $env:TEMP "$Binary-$Target-$PID.exe"
$TmpSum = Join-Path $env:TEMP "$Binary-$Target-$PID.sha256"

Write-Host "=> Downloading installer (1/2)..."
Invoke-WebRequest -Uri $BinUrl -OutFile $TmpBin -UseBasicParsing
Write-Host "=> Downloading checksum (2/2)..."
Invoke-WebRequest -Uri $SumUrl -OutFile $TmpSum -UseBasicParsing

# ── SHA256 verification ───────────────────────────────────────────────────────
Write-Host "=> Verifying checksum..."

$SumLine      = Get-Content $TmpSum -Raw
$ExpectedHash = ($SumLine -split '\s+')[0].Trim().ToUpper()

$ActualHash = (Get-FileHash -Path $TmpBin -Algorithm SHA256).Hash.ToUpper()

if ($ActualHash -ne $ExpectedHash) {
    Remove-Item $TmpBin -ErrorAction SilentlyContinue
    Remove-Item $TmpSum -ErrorAction SilentlyContinue
    Write-Host "error: Checksum mismatch for $Binary-$Target.exe." -ForegroundColor Red
    Write-Host "  Expected: $ExpectedHash" -ForegroundColor Red
    Write-Host "  Got:      $ActualHash" -ForegroundColor Red
    Write-Host "  The download may be corrupt or tampered with. Please retry." -ForegroundColor Red
    exit 1
}

Write-Host "=> Checksum verified." -ForegroundColor Green
Remove-Item $TmpSum -ErrorAction SilentlyContinue

# ── Install binary to per-user bin dir ───────────────────────────────────────
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$InstalledBin = Join-Path $InstallDir "$Binary.exe"
Move-Item -Path $TmpBin -Destination $InstalledBin -Force

Write-Host "=> Installed to $InstalledBin" -ForegroundColor Green

# ── PATH registration via user environment (registry) ────────────────────────
$UserPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")

# Exact-entry comparison: a substring match would be fooled by e.g. ...\bin-old
if (($UserPath -split ';') -notcontains $InstallDir) {
    $NewPath = if ($UserPath) { "$InstallDir;$UserPath" } else { $InstallDir }
    [System.Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Host ""
    Write-Host "=> Added $InstallDir to your user PATH." -ForegroundColor Yellow
    Write-Host "   Restart your terminal (or open a new one) for engrammic to be on PATH."
    Write-Host ""
} else {
    Write-Host "=> $InstallDir is already in PATH"
}

# ── Exec the installed binary with passthrough args ───────────────────────────
# The binary needs a subcommand first: bare flags like `-y` mean `install -y`,
# while a leading word like `selfhost` is taken as the subcommand itself.
Write-Host ""
Write-Host "=> Running installer..." -ForegroundColor Cyan

if ($args.Count -eq 0) {
    & $InstalledBin install
} elseif ($args[0] -like '-*') {
    & $InstalledBin install @args
} else {
    & $InstalledBin @args
}

Write-Host ""
Write-Host "=> Installation complete!" -ForegroundColor Green
