# castle-song.ps1 - sing one song for the companion.
#
#   castle-song.ps1 -Title "<title>" -Style "<style tags>"
#                   -Lyrics "<lyrics text, or a path to a .md/.txt file>"
#                   -OutFolder <folder>
#                   [-Instrumental] [-Model V5]
#                   [-WhatIfCost]                # print the estimate, no request
#
# Uses sunoapi.org with SUNOAPI_ORG_KEY: starts a generation, checks it every
# 10 seconds for up to 10 minutes, then saves each take as <NN>-<slug>.mp3
# (plus <NN>-<slug>.jpg cover art when the service returns one).
# No key: the lyrics and style are saved as <slug>.lyrics.md and the script exits 3.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$Style,
    [string]$Lyrics = '',
    [Parameter(Mandatory = $true)][string]$OutFolder,
    [switch]$Instrumental,
    [string]$Model = 'V5',
    [switch]$WhatIfCost
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$ApiBase = 'https://api.sunoapi.org/api/v1'
# The service requires a callback URL; nothing listens there because this script polls.
$CallbackUrl = 'https://example.com/castle-callback'
$PollSeconds = 10
$TimeoutSeconds = 600

# ROUGH ESTIMATE: one generation (usually two takes) costs a few credits on
# sunoapi.org, around USD 0.06 at common credit prices. Check the member's
# sunoapi.org balance for the real figure.
$cost = 0.06

$slug = Get-CastleSlug -Text $Title -Default 'song'
$folder = [System.IO.Path]::GetFullPath($OutFolder)

if (-not $Instrumental -and [string]::IsNullOrWhiteSpace($Lyrics)) {
    Stop-CastleBadInput 'lyrics are needed unless -Instrumental is set'
}
$lyricsText = ''
if (-not [string]::IsNullOrWhiteSpace($Lyrics)) { $lyricsText = Resolve-CastleText -Value $Lyrics }

if ($WhatIfCost) {
    $kind = if ($Instrumental) { 'instrumental' } else { 'with vocals' }
    Write-CastleEstimate -CostEstimateUsd $cost -Model $Model -Detail "one generation ($kind), usually two takes"
    exit 0
}

$apiKey = Get-CastleKey 'SUNOAPI_ORG_KEY'
if (-not $apiKey) {
    $content = "# $Title`r`n`r`n## Style`r`n`r`n$Style`r`n`r`n- model: $Model`r`n- instrumental: $([bool]$Instrumental)`r`n`r`n## Lyrics`r`n`r`n"
    if ($lyricsText) { $content += $lyricsText } else { $content += '(instrumental - no lyrics)' }
    Write-CastleFallback -Kind song -OutFile (Join-Path $folder "$slug.mp3") -Content $content -Reason 'no sunoapi.org key is set for songs'
}

if (-not (Test-Path -LiteralPath $folder)) { New-Item -ItemType Directory -Path $folder -Force | Out-Null }
$headers = @{ Authorization = "Bearer $apiKey" }
$start = Get-Date

# 1) Start the generation.
$req = @{
    customMode   = $true
    instrumental = [bool]$Instrumental
    model        = $Model
    style        = $Style
    title        = $Title
    callBackUrl  = $CallbackUrl
}
if (-not $Instrumental) { $req['prompt'] = $lyricsText }
$body = ConvertTo-CastleJsonBytes $req
try {
    $url = "$ApiBase/generate"
    Assert-CastleNetworkAllowed $url
    $created = Invoke-RestMethod -Uri $url -Method Post -Headers $headers -Body $body `
        -ContentType 'application/json; charset=utf-8' -TimeoutSec 120
} catch {
    Stop-CastleFailure -What 'starting the song' -ErrorRecord $_
}
if ($created.code -and [int]$created.code -ne 200) {
    Stop-CastleFailure -What 'starting the song' -Message "the service said code $($created.code): $($created.msg)"
}
$taskId = $null
if ($created.data) { $taskId = $created.data.taskId }
if (-not $taskId) { Stop-CastleFailure -What 'starting the song' -Message 'the answer held no taskId' }
Write-CastleLine "Song task $taskId started; checking every $PollSeconds s for up to $([int]($TimeoutSeconds / 60)) min."

# 2) Poll until SUCCESS or a failed status.
$tracks = $null
$status = ''
while ($true) {
    Start-Sleep -Seconds $PollSeconds
    if (((Get-Date) - $start).TotalSeconds -gt $TimeoutSeconds) {
        Stop-CastleFailure -What 'the song task' -Message "still '$status' after $([int]($TimeoutSeconds / 60)) minutes (task $taskId); it may finish later on sunoapi.org"
    }
    try {
        $infoUrl = "$ApiBase/generate/record-info?taskId=$([uri]::EscapeDataString($taskId))"
        Assert-CastleNetworkAllowed $infoUrl
        $info = Invoke-RestMethod -Uri $infoUrl -Method Get -Headers $headers -TimeoutSec 60
    } catch {
        Stop-CastleFailure -What 'checking the song task' -ErrorRecord $_
    }
    $status = ([string]$info.data.status).ToUpperInvariant()
    if ($status -eq 'SUCCESS') {
        $tracks = @($info.data.response.sunoData)
        if ($tracks.Count -lt 1) { Stop-CastleFailure -What 'the song task' -Message "SUCCESS but no tracks came back (task $taskId)" }
        break
    }
    if ($status -match 'FAIL|ERROR|EXCEPTION') {
        $why = if ($info.data.errorMessage) { [string]$info.data.errorMessage } else { '(no detail given)' }
        Stop-CastleFailure -What 'the song task' -Message "$status (task $taskId): $why"
    }
    # PENDING / TEXT_SUCCESS / FIRST_SUCCESS: keep waiting
}

# 3) Download every take (and its cover when present).
$saved = @()
$n = 0
foreach ($t in $tracks) {
    $n++
    $nn = '{0:D2}' -f $n
    $audio = $t.audioUrl
    if (-not $audio) { Write-CastleLine "Take $nn had no audioUrl; skipped."; continue }
    $mp3 = Join-Path $folder "$nn-$slug.mp3"
    try {
        Assert-CastleNetworkAllowed $audio
        Invoke-WebRequest -Uri $audio -OutFile $mp3 -UseBasicParsing -TimeoutSec 300 | Out-Null
        $saved += $mp3
    } catch {
        Stop-CastleFailure -What "downloading take $nn" -ErrorRecord $_
    }
    if ($t.imageUrl) {
        $jpg = Join-Path $folder "$nn-$slug.jpg"
        try {
            Assert-CastleNetworkAllowed $t.imageUrl
            Invoke-WebRequest -Uri $t.imageUrl -OutFile $jpg -UseBasicParsing -TimeoutSec 120 | Out-Null
            $saved += $jpg
        } catch {
            Write-CastleLine ("Cover for take $nn was not saved: " + (Get-CastleErrorText -ErrorRecord $_))
        }
    }
}
if ($saved.Count -lt 1) { Stop-CastleFailure -What 'the song task' -Message "no take had audio to download (task $taskId)" }

Write-CastleResult -SavedPath $saved -CostEstimateUsd $cost -ElapsedSeconds ((Get-Date) - $start).TotalSeconds -Model $Model
exit 0
