# castle-video.ps1 - make one short video clip for the companion.
#
#   castle-video.ps1 -Prompt "<text>" -OutFile <path.mp4>
#                    [-Seconds 5] [-Resolution 480p|720p|1080p]
#                    [-WhatIfCost]               # print the estimate, no request
#
# Uses BytePlus ModelArk Seedance with ARK_API_KEY: creates a generation task,
# checks it every 10 seconds for up to 10 minutes, then downloads the clip.
# No key: the prompt is saved as <name>.prompt.md and the script exits 3.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Prompt,
    [Parameter(Mandatory = $true)][string]$OutFile,
    [ValidateRange(1, 15)][int]$Seconds = 5,
    [ValidateSet('480p', '720p', '1080p')][string]$Resolution = '720p',
    [switch]$WhatIfCost
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$Model = 'dreamina-seedance-2-0-260128'
$BaseUrl = 'https://ark.ap-southeast.bytepluses.com/api/v3/contents/generations/tasks'
$PollSeconds = 10
$TimeoutSeconds = 600

# ModelArk bills video by tokens: roughly seconds x width x height x 24 / 1024.
# Tokens per second and USD per million tokens below are ROUGH ESTIMATES and
# depend on the member's plan or resource pack; the BytePlus console is the truth.
$tokensPerSecond = @{ '480p' = 10000; '720p' = 22000; '1080p' = 49000 }
$usdPerMillion = 7.0
$tokens = $tokensPerSecond[$Resolution] * $Seconds
$cost = $tokens / 1000000.0 * $usdPerMillion

$fullOut = [System.IO.Path]::GetFullPath($OutFile)

if ($WhatIfCost) {
    Write-CastleEstimate -CostEstimateUsd $cost -Model $Model -Detail "$Seconds s at $Resolution, about $tokens tokens at USD $usdPerMillion per million"
    exit 0
}

$apiKey = Get-CastleKey 'ARK_API_KEY'
if (-not $apiKey) {
    $content = "# Video prompt`r`n`r`n" + $Prompt + "`r`n`r`n- seconds: $Seconds`r`n- resolution: $Resolution`r`n- model: $Model"
    Write-CastleFallback -Kind video -OutFile $fullOut -Content $content -Reason 'no Seedance (BytePlus ARK) key is set for video'
}

$fullOut = New-CastleFolderFor -Path $fullOut
$headers = @{ Authorization = "Bearer $apiKey" }
$start = Get-Date

# 1) Create the task.
$body = ConvertTo-CastleJsonBytes @{
    model      = $Model
    content    = @(@{ type = 'text'; text = $Prompt })
    duration   = $Seconds
    resolution = $Resolution
}
try {
    Assert-CastleNetworkAllowed $BaseUrl
    $created = Invoke-RestMethod -Uri $BaseUrl -Method Post -Headers $headers -Body $body `
        -ContentType 'application/json; charset=utf-8' -TimeoutSec 120
} catch {
    Stop-CastleFailure -What 'creating the video task' -ErrorRecord $_
}
$taskId = $created.id
if (-not $taskId) { Stop-CastleFailure -What 'creating the video task' -Message 'the answer held no task id' }
Write-CastleLine "Video task $taskId started; checking every $PollSeconds s for up to $([int]($TimeoutSeconds / 60)) min."

# 2) Poll until succeeded or failed.
$videoUrl = $null
$status = ''
while ($true) {
    Start-Sleep -Seconds $PollSeconds
    if (((Get-Date) - $start).TotalSeconds -gt $TimeoutSeconds) {
        Stop-CastleFailure -What 'the video task' -Message "still '$status' after $([int]($TimeoutSeconds / 60)) minutes (task $taskId); it may finish later on BytePlus"
    }
    try {
        $taskUrl = "$BaseUrl/$taskId"
        Assert-CastleNetworkAllowed $taskUrl
        $task = Invoke-RestMethod -Uri $taskUrl -Method Get -Headers $headers -TimeoutSec 60
    } catch {
        Stop-CastleFailure -What 'checking the video task' -ErrorRecord $_
    }
    $status = ([string]$task.status).ToLowerInvariant()
    if ($status -eq 'succeeded') {
        if ($task.content -and $task.content.video_url) { $videoUrl = [string]$task.content.video_url }
        if (-not $videoUrl) { Stop-CastleFailure -What 'the video task' -Message "succeeded but held no content.video_url (task $taskId)" }
        break
    }
    if ($status -in @('failed', 'cancelled', 'canceled', 'expired')) {
        $why = '(no detail given)'
        if ($task.error) {
            if ($task.error.message) { $why = [string]$task.error.message } else { $why = ($task.error | ConvertTo-Json -Compress -Depth 4) }
        }
        Stop-CastleFailure -What 'the video task' -Message "$status (task $taskId): $why"
    }
    # queued / running: keep waiting
}

# 3) Download the clip.
try {
    Assert-CastleNetworkAllowed $videoUrl
    Invoke-WebRequest -Uri $videoUrl -OutFile $fullOut -UseBasicParsing -TimeoutSec 300 | Out-Null
} catch {
    Stop-CastleFailure -What 'downloading the video' -ErrorRecord $_
}

Write-CastleResult -SavedPath $fullOut -CostEstimateUsd $cost -ElapsedSeconds ((Get-Date) - $start).TotalSeconds -Model $Model
exit 0
