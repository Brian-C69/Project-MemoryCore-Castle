<#
.SYNOPSIS
  Builds an official, signed MemoryCore Castle release by hand. The usual road is the release
  workflow: bump the version (node scripts/bump.mjs x.y.z), commit, push the tag vx.y.z, and
  GitHub builds, signs and publishes it.

.DESCRIPTION
  Only official builds update themselves. This script:
    -Setup     once: makes the updater signing key pair. The PRIVATE key stays on this computer in
               %APPDATA%\MemoryCore Castle Release\ (never in the repository; its text also goes
               into the GitHub secret TAURI_SIGNING_PRIVATE_KEY). The PUBLIC key is written into
               src-tauri\tauri.official.conf.json, which every official build uses.
    (default)  builds the NSIS installer with CASTLE_OFFICIAL=1 and the "official" feature, signs
               the updater artifact, and writes the release files (installer, signature,
               latest.json, notes.md from CHANGELOG.md) into release-out\.
    -Publish   also creates the GitHub release vX.Y.Z with those files (needs the GitHub CLI).

  Members' own builds (npm run tauri build) never contain the updater.

.EXAMPLE
  powershell -File scripts\release.ps1 -Setup
  powershell -File scripts\release.ps1
  powershell -File scripts\release.ps1 -Publish
#>
param(
    [switch]$Setup,
    [switch]$Publish,
    [string]$Repo = "Kiyoraka/Project-MemoryCore-Castle"
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$keyDir = Join-Path $env:APPDATA 'MemoryCore Castle Release'
$keyFile = Join-Path $keyDir 'castle-updater.key'
$pubFile = "$keyFile.pub"
$officialConf = Join-Path $root 'src-tauri\tauri.official.conf.json'

function Write-OfficialConf([string]$pub) {
    $endpoint = "https://github.com/$Repo/releases/latest/download/latest.json"
    $config = [ordered]@{
        bundle  = [ordered]@{ createUpdaterArtifacts = $true }
        plugins = [ordered]@{ updater = [ordered]@{ pubkey = $pub; endpoints = @($endpoint); windows = @{ installMode = 'passive' } } }
    } | ConvertTo-Json -Depth 6
    [System.IO.File]::WriteAllText($officialConf, $config + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

if ($Setup) {
    if (Test-Path $keyFile) { throw "A signing key already exists at $keyFile. Keep it: every installed Castle trusts it." }
    New-Item -ItemType Directory -Force -Path $keyDir | Out-Null
    Push-Location $root
    $ErrorActionPreference = 'Continue'
    try { npx tauri signer generate --ci -w $keyFile } finally { $ErrorActionPreference = 'Stop'; Pop-Location }
    if (-not (Test-Path $pubFile)) { throw "The key pair was not made." }
    Write-OfficialConf (Get-Content $pubFile -Raw).Trim()
    Write-Host "Private key: $keyFile (back it up somewhere safe; never commit it)"
    Write-Host "Public key written into src-tauri\tauri.official.conf.json (commit that file)."
    Write-Host "GitHub secret TAURI_SIGNING_PRIVATE_KEY = the private key file's text."
    exit 0
}

if (-not (Test-Path $keyFile)) { throw "No signing key yet. Run: powershell -File scripts\release.ps1 -Setup" }
if (-not (Test-Path $officialConf)) { throw "src-tauri\tauri.official.conf.json is missing." }
$version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version

$env:CASTLE_OFFICIAL = '1'
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyFile -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
Push-Location $root
$ErrorActionPreference = 'Continue'
try {
    npx tauri build --features official --config src-tauri/tauri.official.conf.json
    if ($LASTEXITCODE -ne 0) { throw "The build failed." }
    node scripts/latest-json.mjs
    if ($LASTEXITCODE -ne 0) { throw "The release files could not be written." }
} finally {
    $ErrorActionPreference = 'Stop'
    Pop-Location
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:CASTLE_OFFICIAL -ErrorAction SilentlyContinue
}

$out = Join-Path $root 'release-out'
if ($Publish) {
    $files = Get-ChildItem $out | Where-Object { $_.Name -ne 'notes.md' } | ForEach-Object { $_.FullName }
    gh release create "v$version" @files --repo $Repo --title "MemoryCore Castle $version" --notes-file (Join-Path $out 'notes.md')
    if ($LASTEXITCODE -ne 0) { throw "The GitHub release could not be created." }
    Write-Host "Published v$version"
}
