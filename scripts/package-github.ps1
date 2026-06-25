#Requires -Version 5.1
<#
.SYNOPSIS
    Assemble a clean, publishable copy of MD -> ALL for GitHub.

.DESCRIPTION
    Copies only the files that make up the shippable project (source, assets,
    build scripts, README) into a standalone folder, leaving behind every
    working artifact and internal note: the .git history, build output, the
    bundled Chromium, target/, backups, and the internal handoff/audit/status
    documents. The result is a self-contained folder you can upload to a fresh
    GitHub repository by hand, with no history attached.

    Selection is an explicit allowlist, not a denylist, so an internal note can
    never leak in by accident. After assembly the script greps the package for
    authorship tells and reports any hit.

.PARAMETER OutputDir
    Where to assemble the package (default: <root>/dist/github).
#>
param(
    [string]$OutputDir = '',
    [switch]$GitInit
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
if (-not $OutputDir) { $OutputDir = Join-Path $root 'dist/github' }

# Only these top-level items are published.
$include = @(
    'Cargo.toml', 'Cargo.lock', 'build.rs', 'README.md', '.gitignore',
    'src', 'crates', 'assets', 'scripts', '.github'
)
# Build output that may live nested inside copied trees: always stripped.
$pruneDirs = @('target', 'out', 'backups', 'chromium', 'dist', '.git')

if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
New-Item -ItemType Directory -Force $OutputDir | Out-Null

foreach ($item in $include) {
    $srcPath = Join-Path $root $item
    if (-not (Test-Path $srcPath)) {
        Write-Host "skip (absent): $item" -ForegroundColor DarkYellow
        continue
    }
    $dest = Join-Path $OutputDir $item
    if (Test-Path $srcPath -PathType Container) {
        Copy-Item $srcPath $dest -Recurse -Force
    } else {
        Copy-Item $srcPath $dest -Force
    }
}

# Prune build artifacts that were copied inside the allowlisted trees.
foreach ($d in $pruneDirs) {
    Get-ChildItem -Path $OutputDir -Recurse -Directory -Filter $d -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item -Recurse -Force $_.FullName }
}

# Safety net: fail loudly if any authorship tell slipped into the package.
$pattern = 'claude|anthropic|co-authored|openai|\bGPT\b'
$hits = Get-ChildItem -Path $OutputDir -Recurse -File -Include *.rs, *.md, *.toml, *.yml, *.yaml |
    Select-String -Pattern $pattern -List -ErrorAction SilentlyContinue
if ($hits) {
    Write-Host "WARNING: authorship tells found in the package:" -ForegroundColor Red
    $hits | ForEach-Object { Write-Host "  $($_.Path):$($_.LineNumber)" -ForegroundColor Red }
} else {
    Write-Host "Clean: no authorship tells in the package." -ForegroundColor Green
}

$size = [math]::Round(((Get-ChildItem -Recurse -File $OutputDir | Measure-Object Length -Sum).Sum) / 1MB, 1)
Write-Host "Package assembled at $OutputDir  ($size MB)" -ForegroundColor Green

if ($GitInit) {
    # Start a fresh repository inside the package: one clean commit, no prior
    # history (so none of the working repo's history reaches GitHub).
    $verMatch = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    $ver = if ($verMatch) { $verMatch.Matches.Groups[1].Value } else { '0.0.0' }
    Push-Location $OutputDir
    try {
        git init -q
        git add -A
        # Author the public release as the company, not the local git identity.
        git -c user.name="Hope 'n Mind" -c user.email="contact@hopenmind.com" commit -q -m "MD -> ALL $ver"
        git branch -M main
        Write-Host "Fresh git repo initialized in $OutputDir (one clean commit, no history)." -ForegroundColor Green
        Write-Host "Next: create an empty GitHub repo, then run there:" -ForegroundColor Cyan
        Write-Host "  git remote add origin <url>" -ForegroundColor Cyan
        Write-Host "  git push -u origin main" -ForegroundColor Cyan
        Write-Host "  git tag v$ver; git push origin v$ver   # triggers the release build" -ForegroundColor Cyan
    }
    finally { Pop-Location }
} else {
    Write-Host "Upload its contents to a fresh GitHub repo by hand, or re-run with -GitInit." -ForegroundColor Cyan
}
