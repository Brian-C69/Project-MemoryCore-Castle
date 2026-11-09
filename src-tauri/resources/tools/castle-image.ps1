# castle-image.ps1 - draw one picture for the companion.
#
#   castle-image.ps1 -Prompt "<text>" -OutFile <path.png>
#                    [-Size 1024x1024|1024x1536|1536x1024]
#                    [-Quality low|medium|high]
#                    [-Reference <image path>]   # edit / restyle from a picture
#                    [-WhatIfCost]               # print the estimate, no request
#
# Uses OpenAI gpt-image-2 with OPENAI_API_KEY (Castle passes that key to
# Claude Code only). When CASTLE_IMAGE=codex, Codex draws with its own image
# tool instead and this script exits 4 without writing anything.
# No key: the prompt is saved as <name>.prompt.md and the script exits 3.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Prompt,
    [Parameter(Mandatory = $true)][string]$OutFile,
    [ValidateSet('1024x1024', '1024x1536', '1536x1024')][string]$Size = '1024x1024',
    [ValidateSet('low', 'medium', 'high')][string]$Quality = 'medium',
    [string]$Reference,
    # a see-through background (cast portraits that stand over a scene); PNG only
    [switch]$Transparent,
    [switch]$WhatIfCost
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$Model = 'gpt-image-2'

# Rough USD per image by size and quality. These are ESTIMATES taken from
# public price lists and may drift; a -Reference edit also bills the input
# image tokens, which this table does not include. The provider bill wins.
$costTable = @{
    '1024x1024_low'    = 0.02
    '1024x1024_medium' = 0.04
    '1024x1024_high'   = 0.19
    '1024x1536_low'    = 0.03
    '1024x1536_medium' = 0.06
    '1024x1536_high'   = 0.25
    '1536x1024_low'    = 0.03
    '1536x1024_medium' = 0.06
    '1536x1024_high'   = 0.25
}
$cost = $costTable["${Size}_${Quality}"]

$fullOut = [System.IO.Path]::GetFullPath($OutFile)

if ($env:CASTLE_IMAGE -eq 'codex') {
    Write-CastleLine "Codex draws pictures with its own image tool - use it and save to $fullOut"
    exit 4
}

if ($WhatIfCost) {
    $mode = if ($Reference) { 'edit with a reference picture (input image tokens extra)' } else { 'new picture' }
    Write-CastleEstimate -CostEstimateUsd $cost -Model $Model -Detail "$Size, $Quality quality, $mode"
    exit 0
}

if ($Reference -and -not (Test-Path -LiteralPath $Reference -PathType Leaf)) {
    Stop-CastleBadInput "reference picture not found: $Reference"
}

$apiKey = Get-CastleKey 'OPENAI_API_KEY'
if (-not $apiKey) {
    $content = "# Picture prompt`r`n`r`n" + $Prompt + "`r`n`r`n- size: $Size`r`n- quality: $Quality"
    if ($Reference) { $content += "`r`n- reference: $Reference" }
    Write-CastleFallback -Kind image -OutFile $fullOut -Content $content -Reason 'no OpenAI key is set for pictures'
}

$fullOut = New-CastleFolderFor -Path $fullOut
$start = Get-Date

if (-not $Reference) {
    # New picture: JSON body. gpt-image models always answer with b64_json, so
    # no response_format is sent (the endpoint rejects it for these models).
    $url = 'https://api.openai.com/v1/images/generations'
    $request = @{
        model   = $Model
        prompt  = $Prompt
        size    = $Size
        quality = $Quality
        n       = 1
    }
    if ($Transparent) { $request.background = 'transparent' }
    $body = ConvertTo-CastleJsonBytes $request
    try {
        Assert-CastleNetworkAllowed $url
        $resp = Invoke-RestMethod -Uri $url -Method Post -Headers @{ Authorization = "Bearer $apiKey" } `
            -Body $body -ContentType 'application/json; charset=utf-8' -TimeoutSec 300
    } catch {
        Stop-CastleFailure -What 'the picture request' -ErrorRecord $_
    }
} else {
    # Edit from a reference picture: multipart form (PS 5.1 has no -Form).
    $url = 'https://api.openai.com/v1/images/edits'
    Add-Type -AssemblyName System.Net.Http
    $client = $null
    $form = $null
    try {
        Assert-CastleNetworkAllowed $url
        $refPath = (Resolve-Path -LiteralPath $Reference).ProviderPath
        $ext = [System.IO.Path]::GetExtension($refPath).ToLowerInvariant()
        $mime = switch ($ext) { '.png' { 'image/png' } '.webp' { 'image/webp' } default { 'image/jpeg' } }

        $client = New-Object System.Net.Http.HttpClient
        $client.Timeout = [TimeSpan]::FromMinutes(5)
        $client.DefaultRequestHeaders.Authorization = New-Object System.Net.Http.Headers.AuthenticationHeaderValue('Bearer', $apiKey)

        $form = New-Object System.Net.Http.MultipartFormDataContent
        $imgBytes = [System.IO.File]::ReadAllBytes($refPath)
        $imgPart = New-Object System.Net.Http.ByteArrayContent(, $imgBytes)
        $imgPart.Headers.ContentType = [System.Net.Http.Headers.MediaTypeHeaderValue]::Parse($mime)
        $form.Add($imgPart, 'image', [System.IO.Path]::GetFileName($refPath))
        $form.Add((New-Object System.Net.Http.StringContent($Prompt, [System.Text.Encoding]::UTF8)), 'prompt')
        $form.Add((New-Object System.Net.Http.StringContent($Model)), 'model')
        $form.Add((New-Object System.Net.Http.StringContent($Size)), 'size')
        $form.Add((New-Object System.Net.Http.StringContent($Quality)), 'quality')
        $form.Add((New-Object System.Net.Http.StringContent('1')), 'n')
        if ($Transparent) { $form.Add((New-Object System.Net.Http.StringContent('transparent')), 'background') }

        $http = $client.PostAsync($url, $form).GetAwaiter().GetResult()
        $raw = $http.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        if (-not $http.IsSuccessStatusCode) {
            $detail = $raw
            try { $j = $raw | ConvertFrom-Json; if ($j.error.message) { $detail = $j.error.message } } catch {}
            Stop-CastleFailure -What 'the picture edit request' -Message ("HTTP $([int]$http.StatusCode) - " + (($detail -replace '\s+', ' ').Trim()))
        }
        $resp = $raw | ConvertFrom-Json
    } catch {
        Stop-CastleFailure -What 'the picture edit request' -ErrorRecord $_
    } finally {
        if ($form) { $form.Dispose() }
        if ($client) { $client.Dispose() }
    }
}

if (-not $resp.data -or @($resp.data).Count -lt 1 -or -not $resp.data[0].b64_json) {
    Stop-CastleFailure -What 'the picture request' -Message 'the answer held no picture data (b64_json missing)'
}
try {
    [System.IO.File]::WriteAllBytes($fullOut, [System.Convert]::FromBase64String($resp.data[0].b64_json))
} catch {
    Stop-CastleFailure -What 'saving the picture' -ErrorRecord $_
}

Write-CastleResult -SavedPath $fullOut -CostEstimateUsd $cost -ElapsedSeconds ((Get-Date) - $start).TotalSeconds -Model $Model
exit 0
