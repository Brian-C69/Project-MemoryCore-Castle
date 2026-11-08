# castle-common.ps1 - shared pieces for the Castle helper scripts.
#
# Dot-sourced by castle-image.ps1, castle-video.ps1, castle-song.ps1 and
# castle-voice.ps1. Windows PowerShell 5.1, ASCII-only source.
#
# Castle hands keys to the companion's CLI only as environment variables.
# Nothing here ever prints a key. Files written are UTF-8 without a BOM.
#
# Exit codes used by the helpers:
#   0  done (or -WhatIfCost estimate printed)
#   1  the API call failed (one-line reason printed, secrets removed)
#   2  bad input (for example a missing reference file)
#   3  no key - a fallback file was saved instead and a notice was left
#   4  the CLI should use its own tool instead (Codex drawing pictures)
#   9  a network call was blocked because CASTLE_NO_NETWORK=1 (tests only)

[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor [System.Net.SecurityProtocolType]::Tls12

$script:CastleUtf8 = New-Object System.Text.UTF8Encoding($false)
$script:CastleKeyNames = @('OPENAI_API_KEY', 'ARK_API_KEY', 'SUNOAPI_ORG_KEY', 'ELEVENLABS_API_KEY')

function Write-CastleLine {
    param([string]$Text)
    [Console]::Out.WriteLine($Text)
}

function Get-CastleKey {
    # Returns the key from the named environment variable, or $null when it is
    # missing, empty, or obviously a placeholder ("your-key-here", "<key>", ...).
    param([Parameter(Mandatory = $true, Position = 0)][string]$Name)
    $value = [Environment]::GetEnvironmentVariable($Name, 'Process')
    if ([string]::IsNullOrWhiteSpace($value)) { return $null }
    $value = $value.Trim().Trim('"').Trim("'").Trim()
    if ($value.Length -lt 8) { return $null }
    if ($value -match '\s') { return $null }
    if ($value -match '\.\.\.') { return $null }
    if ($value -match '^[<\[{(]') { return $null }
    if ($value -match '(?i)^(sk-)?(your|insert|enter|paste|put|add|replace|changeme|change-me|placeholder|example|xxx|todo|tbd|none|null|undefined|empty|missing|key-here|api-key-here)') { return $null }
    if ($value -match '(?i)(here|placeholder|changeme)$') { return $null }
    if ($value -match '^(.)\1+$') { return $null }
    return $value
}

function Protect-Secret {
    # Removes anything that looks like a key from text before it is shown.
    param([Parameter(Position = 0)][AllowNull()][AllowEmptyString()][string]$Text)
    if ([string]::IsNullOrEmpty($Text)) { return '' }
    $out = $Text
    foreach ($n in $script:CastleKeyNames) {
        $v = [Environment]::GetEnvironmentVariable($n, 'Process')
        if (-not [string]::IsNullOrWhiteSpace($v) -and $v.Trim().Length -ge 6) {
            $out = $out.Replace($v.Trim(), '[hidden]')
        }
    }
    $out = [regex]::Replace($out, '(?i)(bearer\s+)[^\s"'',;]+', '$1[hidden]')
    $out = [regex]::Replace($out, '(?i)(xi-api-key|api[_-]?key|authorization|token|secret)("?\s*[:=]\s*"?)[^\s"'',;]+', '$1$2[hidden]')
    $out = [regex]::Replace($out, 'sk-[A-Za-z0-9_\-]{6,}', '[hidden]')
    $out = [regex]::Replace($out, '[A-Za-z0-9_\-]{32,}', '[hidden]')
    return $out
}

function Get-CastleErrorText {
    # One line describing a failed web request: HTTP status + the API's own words.
    param([Parameter(Mandatory = $true)]$ErrorRecord)
    $parts = @()
    $code = $null
    try {
        $resp = $ErrorRecord.Exception.Response
        if ($resp -and $resp.StatusCode) { $code = [int]$resp.StatusCode }
    } catch {}
    if ($code) { $parts += "HTTP $code" }
    $detail = $null
    try {
        if ($ErrorRecord.ErrorDetails -and $ErrorRecord.ErrorDetails.Message) { $detail = $ErrorRecord.ErrorDetails.Message }
    } catch {}
    if (-not $detail) { $detail = $ErrorRecord.Exception.Message }
    if ($detail) {
        try {
            $j = $detail | ConvertFrom-Json -ErrorAction Stop
            if ($j.error -and $j.error.message) { $detail = [string]$j.error.message }
            elseif ($j.error -is [string]) { $detail = $j.error }
            elseif ($j.detail -and $j.detail.message) { $detail = [string]$j.detail.message }
            elseif ($j.detail -is [string]) { $detail = $j.detail }
            elseif ($j.msg) { $detail = [string]$j.msg }
            elseif ($j.message) { $detail = [string]$j.message }
        } catch {}
        $parts += $detail
    }
    $line = ($parts -join ' - ') -replace '\s+', ' '
    $line = Protect-Secret $line.Trim()
    if ($line.Length -gt 400) { $line = $line.Substring(0, 400) + '...' }
    return $line
}

function Stop-CastleFailure {
    # Prints one clear line (secrets removed) and exits 1.
    param(
        [Parameter(Mandatory = $true)][string]$What,
        $ErrorRecord,
        [string]$Message
    )
    $detail = $Message
    if ($ErrorRecord) {
        if ($ErrorRecord.Exception -and $ErrorRecord.Exception.Message -like 'CASTLE_NO_NETWORK*') {
            Write-CastleLine (Protect-Secret $ErrorRecord.Exception.Message)
            exit 9
        }
        $detail = Get-CastleErrorText -ErrorRecord $ErrorRecord
    }
    Write-CastleLine ("Error: $What failed: " + (Protect-Secret $detail))
    exit 1
}

function Stop-CastleBadInput {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-CastleLine ("Error: " + (Protect-Secret $Message))
    exit 2
}

function Assert-CastleNetworkAllowed {
    # Called right before every request. Tests set CASTLE_NO_NETWORK=1 to prove
    # that -WhatIfCost and the no-key fallbacks never reach the network.
    param([string]$Url)
    if ($env:CASTLE_NO_NETWORK -eq '1') {
        throw "CASTLE_NO_NETWORK: blocked a request to $Url"
    }
}

function New-CastleFolderFor {
    # Creates the folder that will hold the given file.
    param([Parameter(Mandatory = $true)][string]$Path)
    $full = [System.IO.Path]::GetFullPath($Path)
    $dir = [System.IO.Path]::GetDirectoryName($full)
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    return $full
}

function Resolve-CastleText {
    # Accepts either the text itself or a path to a UTF-8 text file holding it.
    param([Parameter(Mandatory = $true)][string]$Value)
    $isFile = $false
    try {
        if ($Value.Length -lt 260 -and $Value -notmatch "[\r\n]") {
            $isFile = Test-Path -LiteralPath $Value -PathType Leaf
        }
    } catch { $isFile = $false }
    if ($isFile) {
        return [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $Value).ProviderPath, [System.Text.Encoding]::UTF8)
    }
    return $Value
}

function Get-CastleSlug {
    param([string]$Text, [string]$Default = 'untitled')
    $s = ''
    if ($Text) { $s = ($Text.ToLowerInvariant() -replace '[^a-z0-9]+', '-').Trim('-') }
    if ($s.Length -gt 60) { $s = $s.Substring(0, 60).Trim('-') }
    if (-not $s) { $s = $Default }
    return $s
}

function Get-CastleNow {
    return (Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz')
}

function Write-CastleUtf8File {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, $script:CastleUtf8)
}

function Write-CastleFallback {
    # No key: save what would have been rendered (prompt, lyrics or line) next to
    # the intended output, leave one notice line for Castle, and exit 3.
    param(
        [Parameter(Mandatory = $true)][ValidateSet('image', 'video', 'song', 'voice')][string]$Kind,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Content,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    $suffix = @{ image = '.prompt.md'; video = '.prompt.md'; song = '.lyrics.md'; voice = '.line.txt' }[$Kind]
    $setting = @{ image = 'OPENAI_API_KEY'; video = 'ARK_API_KEY'; song = 'SUNOAPI_ORG_KEY'; voice = 'ELEVENLABS_API_KEY' }[$Kind]

    $full = New-CastleFolderFor -Path $OutFile
    $dir = [System.IO.Path]::GetDirectoryName($full)
    $base = [System.IO.Path]::GetFileNameWithoutExtension($full)
    $saved = Join-Path $dir ($base + $suffix)

    $header = "Saved instead of rendering: $Reason. Add the key in Castle's Settings to make the real thing."
    if ($Kind -eq 'voice') {
        $text = $header + "`r`n`r`n" + $Content + "`r`n"
    } else {
        $text = "> " + $header + "`r`n`r`n" + $Content + "`r`n"
    }
    Write-CastleUtf8File -Path $saved -Text $text

    $appData = $env:APPDATA
    if ([string]::IsNullOrWhiteSpace($appData)) { $appData = [Environment]::GetFolderPath('ApplicationData') }
    $noticeDir = Join-Path $appData 'MemoryCore Castle'
    if (-not (Test-Path -LiteralPath $noticeDir)) { New-Item -ItemType Directory -Path $noticeDir -Force | Out-Null }
    $notice = [ordered]@{
        at       = (Get-CastleNow)
        kind     = $Kind
        file     = $saved
        reason   = $Reason
        settings = $setting
    }
    $line = ($notice | ConvertTo-Json -Compress) + "`n"
    [System.IO.File]::AppendAllText((Join-Path $noticeDir 'notices.jsonl'), $line, $script:CastleUtf8)

    $what = @{ image = 'the picture prompt'; video = 'the video prompt'; song = 'the lyrics and style'; voice = 'the line' }[$Kind]
    Write-CastleLine "No $Kind key is set ($setting), so $what was saved to $saved instead of rendering."
    exit 3
}

function Write-CastleResult {
    # Small result block the companion can read back.
    param(
        [Parameter(Mandatory = $true)][string[]]$SavedPath,
        [double]$CostEstimateUsd,
        [double]$ElapsedSeconds,
        [string]$Model
    )
    Write-CastleLine 'result:'
    foreach ($p in $SavedPath) { Write-CastleLine "  saved_path: $p" }
    Write-CastleLine ("  cost_estimate_usd: " + ([Math]::Round($CostEstimateUsd, 4)).ToString([System.Globalization.CultureInfo]::InvariantCulture))
    Write-CastleLine ("  elapsed_seconds: " + ([Math]::Round($ElapsedSeconds, 1)).ToString([System.Globalization.CultureInfo]::InvariantCulture))
    Write-CastleLine "  model: $Model"
}

function Write-CastleEstimate {
    # Printed by -WhatIfCost; no request is made.
    param([double]$CostEstimateUsd, [string]$Model, [string]$Detail)
    Write-CastleLine 'estimate (no request made):'
    Write-CastleLine ("  cost_estimate_usd: " + ([Math]::Round($CostEstimateUsd, 4)).ToString([System.Globalization.CultureInfo]::InvariantCulture))
    Write-CastleLine "  model: $Model"
    if ($Detail) { Write-CastleLine "  basis: $Detail" }
    Write-CastleLine '  note: rough estimate only; the provider bill is the truth.'
}

function ConvertTo-CastleJsonBytes {
    # PS 5.1 sends string bodies as Latin-1; send UTF-8 bytes so any language survives.
    param([Parameter(Mandatory = $true)]$Object)
    $json = $Object | ConvertTo-Json -Depth 10 -Compress
    return [System.Text.Encoding]::UTF8.GetBytes($json)
}
