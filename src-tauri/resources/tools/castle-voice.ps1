# castle-voice.ps1 - speak one line aloud for the companion.
#
#   castle-voice.ps1 -Text "<line, or a path to a UTF-8 text file>" -OutFile <path.mp3>
#                    [-VoiceId <ElevenLabs voice id>]
#                    [-WhatIfCost]               # print the estimate, no request
#
# Uses ElevenLabs text-to-speech (eleven_multilingual_v2) with ELEVENLABS_API_KEY.
# No key: the line is saved as <name>.line.txt and the script exits 3.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Text,
    [Parameter(Mandatory = $true)][string]$OutFile,
    [string]$VoiceId = '21m00Tcm4TlvDq8ikWAM',
    [switch]$WhatIfCost
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$Model = 'eleven_multilingual_v2'

$line = Resolve-CastleText -Value $Text
if ([string]::IsNullOrWhiteSpace($line)) { Stop-CastleBadInput 'the text to speak is empty' }
if ([string]::IsNullOrWhiteSpace($VoiceId)) { Stop-CastleBadInput 'the voice id is empty' }

# ROUGH ESTIMATE: ElevenLabs bills by characters; on common plans this model
# works out near USD 0.20 per 1000 characters. The member's plan decides.
$usdPer1000 = 0.20
$cost = $line.Length / 1000.0 * $usdPer1000

$fullOut = [System.IO.Path]::GetFullPath($OutFile)

if ($WhatIfCost) {
    Write-CastleEstimate -CostEstimateUsd $cost -Model $Model -Detail "$($line.Length) characters at about USD $usdPer1000 per 1000"
    exit 0
}

$apiKey = Get-CastleKey 'ELEVENLABS_API_KEY'
if (-not $apiKey) {
    Write-CastleFallback -Kind voice -OutFile $fullOut -Content $line -Reason 'no ElevenLabs key is set for voice'
}

$fullOut = New-CastleFolderFor -Path $fullOut
$start = Get-Date

$url = 'https://api.elevenlabs.io/v1/text-to-speech/' + [uri]::EscapeDataString($VoiceId)
$body = ConvertTo-CastleJsonBytes @{ text = $line; model_id = $Model }
$existedBefore = Test-Path -LiteralPath $fullOut
try {
    Assert-CastleNetworkAllowed $url
    Invoke-WebRequest -Uri $url -Method Post -UseBasicParsing -TimeoutSec 180 `
        -Headers @{ 'xi-api-key' = $apiKey; Accept = 'audio/mpeg' } `
        -Body $body -ContentType 'application/json; charset=utf-8' -OutFile $fullOut | Out-Null
} catch {
    if (-not $existedBefore -and (Test-Path -LiteralPath $fullOut)) {
        # A failed request must not leave a half-written or error-body .mp3 behind.
        Remove-Item -LiteralPath $fullOut -Force -ErrorAction SilentlyContinue
    }
    Stop-CastleFailure -What 'the voice request' -ErrorRecord $_
}

Write-CastleResult -SavedPath $fullOut -CostEstimateUsd $cost -ElapsedSeconds ((Get-Date) - $start).TotalSeconds -Model $Model
exit 0
