<#
.SYNOPSIS
  Builds an official, signed MemoryCore Castle release and (with -Publish) puts it on GitHub.

.DESCRIPTION
  Only official builds update themselves. This script:
    -Setup     once: makes the updater signing key pair. The PRIVATE key stays on this computer in
               %APPDATA%\MemoryCore Castle Release\ (never in the repository); the PUBLIC key is
               printed and saved beside it, and goes into every official build.
    (default)  builds the NSIS installer with CASTLE_OFFICIAL=1 and the "official" feature, signs
               the updater artifact, and writes latest.json next to it.
    -Publish   also creates the GitHub release vX.Y.Z on the public repository with the
               installer, its signature and latest.json (needs the GitHub CLI, signed in).

  Members' own builds (npm run tauri build) never contain the updater.

.EXAMPLE
  powershell -File scripts\release.ps1 -Setup
  powershell -File scripts\release.ps1 -Notes "First release"
  powershell -File scripts\release.ps1 -Notes "First release" -Publish
#>
param(
    [switch]$Setup,
    [switch]$Publish,
    [string]$Notes = "",
    [string]$Repo = "Kiyoraka/Project-MemoryCore-Castle"
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$keyDir = Join-Path $env:APPDATA 'MemoryCore Castle Release'
$keyFile = Join-Path $keyDir 'castle-updater.key'
$pubFile = "$keyFile.pub"

if ($Setup) {
    if (Test-Path $keyFile) { throw "A signing key already exists at $keyFile. Keep it: every installed Castle trusts it." }
    New-Item -ItemType Directory -Force -Path $keyDir | Out-Null
    Push-Location $root
    try { npx tauri signer generate --ci -w $keyFile } finally { Pop-Location }
    Write-Host "Private key: $keyFile (back it up somewhere safe; never commit it)"
    Write-Host "Public key:  $(Get-Content $pubFile -Raw)"
    exit 0
}

if (-not (Test-Path $keyFile)) { throw "No signing key yet. Run: powershell -File scripts\release.ps1 -Setup" }
$version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version
$pub = (Get-Content $pubFile -Raw).Trim()
$endpoint = "https://github.com/$Repo/releases/latest/download/latest.json"
$config = @{
    bundle  = @{ createUpdaterArtifacts = $true }
    plugins = @{ updater = @{ pubkey = $pub; endpoints = @($endpoint); windows = @{ installMode = 'passive' } } }
} | ConvertTo-Json -Depth 6 -Compress
$configFile = Join-Path $env:TEMP "castle-release-$version.json"
[System.IO.File]::WriteAllText($configFile, $config, (New-Object System.Text.UTF8Encoding($false)))

$env:CASTLE_OFFICIAL = '1'
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyFile -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
Push-Location $root
try {
    npm run tauri build -- --features official --config $configFile
    if ($LASTEXITCODE -ne 0) { throw "The build failed." }
} finally {
    Pop-Location
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
}

$nsis = Join-Path $root 'src-tauri\target\release\bundle\nsis'
$installer = Get-ChildItem $nsis -Filter "*_${version}_*setup.exe" | Select-Object -First 1
$signature = Get-ChildItem $nsis -Filter "*_${version}_*setup.exe.sig" | Select-Object -First 1
if (-not $installer -or -not $signature) { throw "The installer or its signature was not found in $nsis." }
$latest = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = @{ 'windows-x86_64' = @{
        signature = (Get-Content $signature.FullName -Raw).Trim()
        url       = "https://github.com/$Repo/releases/download/v$version/$($installer.Name)"
    } }
}
$latestFile = Join-Path $nsis 'latest.json'
[System.IO.File]::WriteAllText($latestFile, ($latest | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Built $($installer.Name) and latest.json in $nsis"

if ($Publish) {
    gh release create "v$version" $installer.FullName $signature.FullName $latestFile --repo $Repo --title "MemoryCore Castle $version" --notes $Notes
    if ($LASTEXITCODE -ne 0) { throw "The GitHub release could not be created." }
    Write-Host "Published v$version"
}
