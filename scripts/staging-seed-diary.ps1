<#
  Staging seed: adds the sample diary entries in scripts\staging-diary to the staging memory's
  database, through Castle's own door (memorycore-castle.exe --memory diary add), so they are
  saved exactly the way the companion saves one. For checking the DIARY room's three designs.

  Each file in scripts\staging-diary: line 1 the date (YYYY-MM-DD), line 2 the title, then the
  entry. Every seeded row carries a mark, so -Remove takes out exactly these rows and nothing
  the companion or the member wrote.

  Refuses unless:
    - the memory folder in Castle's settings is one of the staging folders named below;
    - that folder's memory is in a database (its .env names one).

  Usage:
    .\scripts\staging-seed-diary.ps1            add the entries (removes earlier seeds first)
    .\scripts\staging-seed-diary.ps1 -Remove    take the seed entries out again
    .\scripts\staging-seed-diary.ps1 -Exe <path to memorycore-castle.exe>
#>
param(
  [switch]$Remove,
  [string]$Exe
)

$ErrorActionPreference = 'Stop'

# The only folders this script will ever write a seed into. Never a real memory.
$StagingFolders = @(
  'Poject AI Memory Architecture - Copy'
)

$repo = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $repo 'src-tauri\target\debug\memorycore-castle.exe' }
if (-not (Test-Path $Exe)) {
  Write-Host "Castle's program was not found at $Exe. Run npm run dev once, or pass -Exe."
  exit 1
}

$settingsPath = Join-Path $env:APPDATA 'com.memorycore.castle\settings.json'
if (-not (Test-Path $settingsPath)) {
  Write-Host 'Castle has no settings yet. Nothing to seed.'
  exit 1
}
$folder = (Get-Content -Raw -Encoding UTF8 $settingsPath | ConvertFrom-Json).memoryFolder
if (-not $folder) {
  Write-Host 'Castle has no memory folder chosen. Nothing to seed.'
  exit 1
}
$leaf = Split-Path -Leaf $folder
if ($StagingFolders -notcontains $leaf) {
  Write-Host "Refused: Castle points at '$leaf', which is not a staging folder."
  Write-Host "Staging folders: $($StagingFolders -join ', ')"
  exit 1
}
$envFile = Join-Path $folder '.env'
if (-not ((Test-Path $envFile) -and (Select-String -Path $envFile -Pattern '^MEMORY_DB_NAME=' -Quiet))) {
  Write-Host 'This memory is not in a database yet. Move it in first (Settings > Database), then run this again.'
  exit 1
}
$database = (Select-String -Path $envFile -Pattern '^MEMORY_DB_NAME=(.+)$').Matches[0].Groups[1].Value

# earlier seeds go first, so running it twice never doubles them
$gone = & $Exe --memory diary unseed --memory-folder $folder
if ($LASTEXITCODE -ne 0) {
  Write-Host "The database did not answer: $gone"
  Write-Host 'Start it (Settings > Database, or XAMPP), then run this again.'
  exit 1
}
if ($Remove) {
  Write-Host "$gone ($database)"
  exit 0
}

$entries = Get-ChildItem -Path (Join-Path $PSScriptRoot 'staging-diary') -Filter '*.md' | Sort-Object Name
$temp = Join-Path $env:TEMP ("castle-seed-" + [guid]::NewGuid().ToString('N') + '.md')
$added = 0
try {
  foreach ($entry in $entries) {
    $lines = [System.IO.File]::ReadAllLines($entry.FullName, [System.Text.Encoding]::UTF8)
    $date = $lines[0].Trim()
    $title = $lines[1].Trim()
    $body = ($lines | Select-Object -Skip 2) -join "`n"
    [System.IO.File]::WriteAllText($temp, $body.Trim() + "`n", (New-Object System.Text.UTF8Encoding($false)))
    $result = & $Exe --memory diary add --date $date --title $title --file $temp --seed $entry.BaseName --memory-folder $folder
    if ($LASTEXITCODE -ne 0) {
      Write-Host "Stopped at $($entry.Name): $result"
      exit 1
    }
    $added += 1
  }
}
finally {
  Remove-Item -Force -ErrorAction SilentlyContinue $temp
}

Write-Host "$added seed entries added to $database"
Write-Host 'Open the DIARY room in Castle (List, Calendar, Book). Take them out again with -Remove.'
