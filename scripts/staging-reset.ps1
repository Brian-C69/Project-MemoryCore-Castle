<#
  Staging reset: puts Castle back to "a member who just updated" so the database offer and the
  move can be tested again from the start.

  It only edits Castle's own settings.json. It never touches a memory folder or a database:
  the memory goes back to files with Castle's own button first
  (Settings > Database > Move my memory back to files).

  Refuses unless:
    - Castle is closed (it keeps its settings in memory and would write them back);
    - the memory folder in Castle's settings is one of the staging folders named below;
    - that folder's memory is in files again (its .env names no memory database).

  Usage:
    .\scripts\staging-reset.ps1                 the offer will be asked again
    .\scripts\staging-reset.ps1 -ForgetEngine   also forget which database Castle uses
                                                (needed for the offer when an existing
                                                 server such as XAMPP was chosen)
#>
param(
  [switch]$ForgetEngine
)

$ErrorActionPreference = 'Stop'

# The only folders this script will ever reset Castle for. Never a real memory.
$StagingFolders = @(
  'Poject AI Memory Architecture - Copy'
)

$settingsPath = Join-Path $env:APPDATA 'com.memorycore.castle\settings.json'
if (-not (Test-Path $settingsPath)) {
  Write-Host "Castle has no settings yet ($settingsPath). Nothing to reset."
  exit 1
}

if (Get-Process -Name 'memorycore-castle' -ErrorAction SilentlyContinue) {
  Write-Host 'Castle is running. Quit it from the tray first, then run this again.'
  exit 1
}

$settings = Get-Content -Raw -Encoding UTF8 $settingsPath | ConvertFrom-Json
$folder = $settings.memoryFolder
if (-not $folder) {
  Write-Host 'Castle has no memory folder chosen. Nothing to reset.'
  exit 1
}
$leaf = Split-Path -Leaf $folder
if ($StagingFolders -notcontains $leaf) {
  Write-Host "Refused: Castle points at '$leaf', which is not a staging folder."
  Write-Host "Staging folders: $($StagingFolders -join ', ')"
  exit 1
}

$envFile = Join-Path $folder '.env'
if ((Test-Path $envFile) -and (Select-String -Path $envFile -Pattern '^MEMORY_DB_NAME=' -Quiet)) {
  Write-Host 'This memory is still in the database.'
  Write-Host 'Open Castle > Settings > Database > "Move my memory back to files", quit Castle, then run this again.'
  exit 1
}

$settings | Add-Member -NotePropertyName dbOfferSeen -NotePropertyValue $false -Force
if ($ForgetEngine) {
  $settings | Add-Member -NotePropertyName dbPort -NotePropertyValue $null -Force
  $settings | Add-Member -NotePropertyName dbExternal -NotePropertyValue $false -Force
}

$json = $settings | ConvertTo-Json -Depth 20
# Castle reads UTF-8 without a byte order mark
[System.IO.File]::WriteAllText($settingsPath, $json, (New-Object System.Text.UTF8Encoding($false)))

Write-Host "Reset for staging folder '$leaf':"
Write-Host '  - the database offer will be asked again'
if ($ForgetEngine) { Write-Host '  - Castle has forgotten which database it used' }
Write-Host 'Open Castle to start the run (Project Resources\staging-checklist.md).'
