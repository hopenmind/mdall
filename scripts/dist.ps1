#Requires -Version 5.1
<#
.SYNOPSIS
    Builds a distributable package for MD -> ALL.

.DESCRIPTION
    1. Runs `cargo build --release`
    2. Creates dist/mdall/ with the exe + the chromium/ folder
    3. Optionally zips the result for distribution

.PARAMETER SkipBuild
    Skip cargo build (use existing target/release/mdall.exe).

.PARAMETER Zip
    Also produce dist/mdall-<version>.zip.

.PARAMETER OutputDir
    Override output directory (default: <project-root>/dist/mdall/).
#>
param(
    [switch]$SkipBuild,
    [switch]$Zip,
    [string]$OutputDir = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Bundle a default spell-check dictionary (en_US) plus its license, so the app
# ships with spell checking working out of the box. Files land in dictionaries/
# next to the exe, where the app auto-loads them at startup.
function Get-DefaultDictionary {
    param([string]$DictDir)
    New-Item -ItemType Directory -Force $DictDir | Out-Null
    $base = 'https://raw.githubusercontent.com/wooorm/dictionaries/main/dictionaries/en'
    $files = @{
        'en_US.dic'         = "$base/index.dic"
        'en_US.aff'         = "$base/index.aff"
        'en_US.license.txt' = "$base/license"
    }
    foreach ($name in $files.Keys) {
        $dest = Join-Path $DictDir $name
        if (-not (Test-Path $dest)) {
            try {
                $ProgressPreference = "SilentlyContinue"
                Invoke-WebRequest -Uri $files[$name] -OutFile $dest -TimeoutSec 60
                $ProgressPreference = "Continue"
            } catch {
                Write-Host "  [!] dictionary fetch failed: $name ($_)" -ForegroundColor Yellow
            }
        }
    }
}

$ProjectRoot  = Split-Path $PSScriptRoot -Parent
$ChromiumDir  = Join-Path $ProjectRoot "chromium"
$ExePath      = Join-Path $ProjectRoot "target\release\mdall.exe"

# Read version from Cargo.toml
$CargoToml    = Get-Content (Join-Path $ProjectRoot "Cargo.toml") -Raw
$Version      = if ($CargoToml -match 'version\s*=\s*"([^"]+)"') { $Matches[1] } else { "unknown" }

$DistRoot     = if ($OutputDir) { $OutputDir } else { Join-Path $ProjectRoot "dist\mdall" }
$DistChromium = Join-Path $DistRoot "chromium"

Write-Host ""
Write-Host "  MD -> ALL — Distribution Packager v$Version" -ForegroundColor Cyan
Write-Host "  Output: $DistRoot"
Write-Host ""

# ── Preflight checks ─────────────────────────────────────────────────────────
$ChromeExe = Join-Path $ChromiumDir "chrome.exe"
if (-not (Test-Path $ChromeExe)) {
    Write-Host "  [!] chromium/chrome.exe not found." -ForegroundColor Red
    Write-Host "      Run: .\scripts\setup-chromium.ps1" -ForegroundColor Yellow
    Write-Host "      then retry dist.ps1."
    exit 1
}

$ChromiumVersion = (Get-Content (Join-Path $ChromiumDir "VERSION.txt") -ErrorAction SilentlyContinue) ?? "(unknown)"
Write-Host "  Bundled Chromium: $ChromiumVersion"

# ── Build ─────────────────────────────────────────────────────────────────────
if (-not $SkipBuild) {
    Write-Host "  Building release binary..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
    Write-Host "  Build OK" -ForegroundColor Green
} else {
    if (-not (Test-Path $ExePath)) {
        Write-Error "  -SkipBuild specified but $ExePath does not exist."
    }
    Write-Host "  [skip] Using existing release binary."
}

# ── Assemble dist folder ──────────────────────────────────────────────────────
Write-Host "  Assembling dist package..." -ForegroundColor Yellow

if (Test-Path $DistRoot) { Remove-Item -Recurse -Force $DistRoot }
New-Item -ItemType Directory -Path $DistRoot -Force | Out-Null

# Main executable
Copy-Item $ExePath $DistRoot

# Chromium (portable folder — full copy, ~180 MB)
Write-Host "  Copying chromium/ (~180 MB, please wait)..."
$ProgressPreference = "SilentlyContinue"
Copy-Item -Recurse $ChromiumDir $DistChromium
$ProgressPreference = "Continue"

# Default dictionary + license (spell check works out of the box).
Write-Host "  Bundling default en_US dictionary + license..."
Get-DefaultDictionary (Join-Path $DistRoot "dictionaries")

# Optional: README / license stub
$ReadmePath = Join-Path $DistRoot "README.txt"
@"
MD -> ALL v$Version
===================

Markdown editor with KaTeX math rendering and high-quality PDF export.

Included:
  mdall.exe     — main application
  chromium/         — ungoogled-chromium $ChromiumVersion (portable, privacy-hardened)

Usage:
  Double-click mdall.exe to launch.

PDF export quality (automatic cascade):
  1. Bundled ungoogled-chromium (best quality, pixel-perfect KaTeX)
  2. Pure-Rust Typst engine (offline fallback, zero system deps)
  3. Unicode approximation (last resort)

ungoogled-chromium is distributed under the BSD-3-Clause license.
Sources: https://github.com/ungoogled-software/ungoogled-chromium-windows
"@ | Set-Content $ReadmePath -Encoding UTF8

# ── Summary ───────────────────────────────────────────────────────────────────
$TotalSize = (Get-ChildItem $DistRoot -Recurse | Measure-Object -Property Length -Sum).Sum
$TotalMB   = [math]::Round($TotalSize / 1MB, 1)

Write-Host "  Package assembled: $TotalMB MB" -ForegroundColor Green
Write-Host "  Location: $DistRoot"

# ── Zip (optional) ────────────────────────────────────────────────────────────
if ($Zip) {
    $ZipPath = Join-Path (Split-Path $DistRoot -Parent) "mdall-$Version-windows-x64.zip"
    Write-Host "  Creating ZIP: $ZipPath ..." -ForegroundColor Yellow
    if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($DistRoot, $ZipPath)
    $ZipMB = [math]::Round((Get-Item $ZipPath).Length / 1MB, 1)
    Write-Host "  ZIP ready: $ZipMB MB → $ZipPath" -ForegroundColor Green
}

Write-Host ""
Write-Host "  Done. Distribute the contents of: $DistRoot" -ForegroundColor Cyan
Write-Host ""
