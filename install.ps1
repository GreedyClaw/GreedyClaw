#!/usr/bin/env pwsh
# GreedyClaw Installer for Windows
# Usage: irm https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$Version = "0.1.0"
$Repo = "GreedyClaw/GreedyClaw"

Write-Host ""
Write-Host "  GreedyClaw Installer v$Version" -ForegroundColor Green
Write-Host "  AI-Native Trading Execution Gateway" -ForegroundColor DarkGray
Write-Host ""

# ── Step 1: Check prerequisites ────────────────────────────────────

Write-Host "[1/5] Checking prerequisites..." -ForegroundColor Cyan

# Check for Rust
$hasRust = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $hasRust) {
    Write-Host "  Rust not found. Installing via rustup..." -ForegroundColor Yellow
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupExe = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe
    & $rustupExe -y --default-toolchain stable
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    Write-Host "  Rust installed!" -ForegroundColor Green
} else {
    $rustVersion = (cargo --version) -replace 'cargo ', ''
    Write-Host "  Rust: $rustVersion" -ForegroundColor Green
}

# Check for Python (needed for MT5/CCXT bridges)
$hasPython = Get-Command python -ErrorAction SilentlyContinue
if ($hasPython) {
    $pyVersion = (python --version 2>&1) -replace 'Python ', ''
    Write-Host "  Python: $pyVersion (MT5/CCXT bridges available)" -ForegroundColor Green
} else {
    Write-Host "  Python: not found (optional — needed for MT5/CCXT bridges)" -ForegroundColor Yellow
}

# Check for protoc
$hasProtoc = Get-Command protoc -ErrorAction SilentlyContinue
if (-not $hasProtoc) {
    Write-Host "  protoc not found. Downloading..." -ForegroundColor Yellow
    $protocUrl = "https://github.com/protocolbuffers/protobuf/releases/download/v28.3/protoc-28.3-win64.zip"
    $protocZip = "$env:TEMP\protoc.zip"
    $protocDir = "$env:LOCALAPPDATA\protoc"
    Invoke-WebRequest -Uri $protocUrl -OutFile $protocZip
    Expand-Archive -Path $protocZip -DestinationPath $protocDir -Force
    $env:PROTOC = "$protocDir\bin\protoc.exe"
    $env:PATH = "$protocDir\bin;$env:PATH"
    Write-Host "  protoc installed to $protocDir" -ForegroundColor Green
} else {
    Write-Host "  protoc: found" -ForegroundColor Green
}

# ── Step 2: Clone/update repo ──────────────────────────────────────

Write-Host ""
Write-Host "[2/5] Getting GreedyClaw source..." -ForegroundColor Cyan

$installDir = "$env:USERPROFILE\.greedyclaw\src"
if (Test-Path "$installDir\.git") {
    Write-Host "  Updating existing installation..."
    Push-Location $installDir
    git pull --ff-only 2>$null
    Pop-Location
} else {
    Write-Host "  Cloning from GitHub..."
    New-Item -ItemType Directory -Path (Split-Path $installDir) -Force | Out-Null
    git clone "https://github.com/$Repo.git" $installDir 2>$null
}

# ── Step 3: Build ──────────────────────────────────────────────────

Write-Host ""
Write-Host "[3/5] Building GreedyClaw (release mode)..." -ForegroundColor Cyan

Push-Location $installDir
cargo build --release 2>&1
Pop-Location

$binary = "$installDir\target\release\greedyclaw.exe"
if (-not (Test-Path $binary)) {
    Write-Host "  Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "  Built: $binary" -ForegroundColor Green

# ── Step 4: Install to PATH ────────────────────────────────────────

Write-Host ""
Write-Host "[4/5] Installing to PATH..." -ForegroundColor Cyan

$binDir = "$env:USERPROFILE\.greedyclaw\bin"
New-Item -ItemType Directory -Path $binDir -Force | Out-Null
Copy-Item $binary "$binDir\greedyclaw.exe" -Force

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$binDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$binDir;$userPath", "User")
    $env:PATH = "$binDir;$env:PATH"
    Write-Host "  Added $binDir to PATH" -ForegroundColor Green
}

# ── Step 5: Initialize config ──────────────────────────────────────

Write-Host ""
Write-Host "[5/5] Initializing configuration..." -ForegroundColor Cyan

& "$binDir\greedyclaw.exe" init

# ── Step 6: Install Python bridges (optional) ──────────────────────

if ($hasPython) {
    Write-Host ""
    Write-Host "Installing Python bridge dependencies..." -ForegroundColor Cyan
    $bridgeDir = "$installDir\mt5-bridge"
    python -m pip install -r "$bridgeDir\requirements.txt" --quiet 2>$null
    python -m pip install ccxt --quiet 2>$null
    Write-Host "  Python bridges ready!" -ForegroundColor Green
}

# ── Done ────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "  GreedyClaw installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "  Quick start:" -ForegroundColor White
Write-Host "    1. Edit ~/.greedyclaw/.env — set your API keys" -ForegroundColor DarkGray
Write-Host "    2. Edit ~/.greedyclaw/config.toml — choose exchange" -ForegroundColor DarkGray
Write-Host "    3. greedyclaw serve" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Supported exchanges:" -ForegroundColor White
Write-Host "    binance, pumpfun, pumpswap, mt5" -ForegroundColor DarkGray
Write-Host "    + 100 more via CCXT: bybit, okx, kraken, coinbase..." -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Dashboard: http://127.0.0.1:7878/dashboard" -ForegroundColor Yellow
Write-Host ""
