# castle-beat.ps1 - write one beat card for Castle's STORY stage.
#
#   castle-beat.ps1 next -World <world-folder>
#   castle-beat.ps1 card -World <world-folder> -Json '<card json>' (or a path to a .json file)
#
# The stage (the visual novel the member sees) reads the newest card in <world>/stage/. A card:
#   {"chapter": "...", "scene": "scenes/03-x.png",
#    "lines": [{"speaker": "", "text": "..."}, {"speaker": "Nova", "text": "...", "voice": "stage/voice/0007-1.mp3"}],
#    "choices": ["...", "...", "(Say something)"]}
# The helper checks the card, numbers it and writes it whole, so the stage never reads half a card.
#
# Windows PowerShell 5.1, ASCII-only source. Exit codes: 0 written (or the number printed),
# 2 the card was refused (one line says why).
param(
    [Parameter(Position = 0)][ValidateSet('next', 'card')][string]$Verb = 'card',
    [Parameter(Mandatory = $true)][string]$World,
    [string]$Json
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$TopKeys = @('beat', 'chapter', 'scene', 'lines', 'choices')
$LineKeys = @('speaker', 'text', 'voice')

function Get-WorldFolder {
    param([string]$Name)
    if ([string]::IsNullOrWhiteSpace($Name) -or $Name -match '\.\.') { Stop-CastleBadInput 'Name the world folder.' }
    $full = [System.IO.Path]::GetFullPath((Join-Path (Get-Location).Path $Name))
    if (-not (Test-Path -LiteralPath $full -PathType Container)) { Stop-CastleBadInput "There is no world folder '$Name' here." }
    return $full
}

function Get-NextBeat {
    param([string]$Folder)
    $stage = Join-Path $Folder 'stage'
    $max = 0
    if (Test-Path -LiteralPath $stage) {
        foreach ($f in Get-ChildItem -LiteralPath $stage -Filter 'beat-*.json' -File) {
            if ($f.BaseName -match '^beat-(\d+)$' -and [int]$Matches[1] -gt $max) { $max = [int]$Matches[1] }
        }
    }
    return $max + 1
}

function Test-InsideWorld {
    # a path inside the world folder, with forward or back slashes, never leaving it
    param([string]$Path)
    return -not ([string]::IsNullOrWhiteSpace($Path) -or $Path -match '\.\.' -or $Path -match '^[A-Za-z]:' -or $Path.StartsWith('/') -or $Path.StartsWith('\'))
}

$folder = Get-WorldFolder $World
if ($Verb -eq 'next') {
    Write-CastleLine ('{0:D4}' -f (Get-NextBeat $folder))
    exit 0
}

if ([string]::IsNullOrWhiteSpace($Json)) { Stop-CastleBadInput 'Give the card as -Json.' }
$text = Resolve-CastleText -Value $Json
try { $card = $text | ConvertFrom-Json } catch { Stop-CastleBadInput 'The card is not valid JSON.' }
if ($null -eq $card -or $card -is [array]) { Stop-CastleBadInput 'The card must be one JSON object.' }

foreach ($key in $card.PSObject.Properties.Name) {
    if ($TopKeys -notcontains $key) { Stop-CastleBadInput "The card has an unknown key '$key' (allowed: $($TopKeys -join ', '))." }
}
$lines = @($card.lines)
if (-not $card.lines -or $lines.Count -eq 0) { Stop-CastleBadInput 'The card needs at least one line.' }
foreach ($line in $lines) {
    foreach ($key in $line.PSObject.Properties.Name) {
        if ($LineKeys -notcontains $key) { Stop-CastleBadInput "A line has an unknown key '$key' (allowed: $($LineKeys -join ', '))." }
    }
    if ([string]::IsNullOrWhiteSpace([string]$line.text)) { Stop-CastleBadInput 'Every line needs its text.' }
    if ($line.voice -and -not (Test-InsideWorld ([string]$line.voice))) { Stop-CastleBadInput 'A voice must be a file inside the world folder.' }
}
$choices = @()
if ($null -ne $card.choices) { $choices = @($card.choices) }
if ($choices.Count -eq 1 -or $choices.Count -gt 5) { Stop-CastleBadInput 'Give 2 to 5 choices (or none, only on the very last beat).' }
foreach ($c in $choices) { if ([string]::IsNullOrWhiteSpace([string]$c)) { Stop-CastleBadInput 'A choice is empty.' } }
if ($card.scene -and -not (Test-InsideWorld ([string]$card.scene))) { Stop-CastleBadInput 'The scene must be a picture inside the world folder.' }

$number = Get-NextBeat $folder
$card | Add-Member -NotePropertyName beat -NotePropertyValue $number -Force
$stage = Join-Path $folder 'stage'
New-Item -ItemType Directory -Force -Path $stage | Out-Null
$target = Join-Path $stage ('beat-{0:D4}.json' -f $number)
$partial = "$target.partial"
Write-CastleUtf8File -Path $partial -Text ($card | ConvertTo-Json -Depth 6)
Move-Item -LiteralPath $partial -Destination $target -Force
Write-CastleLine ("beat {0} written: stage/beat-{0:D4}.json" -f $number)
exit 0
