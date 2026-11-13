# castle-statusline.ps1 - the status line of a Castle session.
#
# Claude Code runs this after each turn and pipes its session as JSON on stdin
# (model, transcript_path and, in newer versions, context_window). It prints one
# line:   <model> | NN% | 123.4k tokens
# - how much of the context window the conversation now fills. Castle's Chat
# reads that line from the Log screen for its context meter.
#
# Nothing personal is read or written: only the JSON and the session's own
# transcript. Windows PowerShell 5.1, ASCII-only source. Never fails loudly - a
# status line that cannot be worked out prints "--".

[Console]::OutputEncoding = [Text.Encoding]::UTF8

function Get-TokenText {
    param([double]$Tokens)
    $culture = [Globalization.CultureInfo]::InvariantCulture
    if ($Tokens -ge 1000000) { return ([math]::Round($Tokens / 1000000, 1)).ToString($culture) + 'M' }
    if ($Tokens -ge 1000) { return ([math]::Round($Tokens / 1000, 1)).ToString($culture) + 'k' }
    return ([math]::Round($Tokens)).ToString($culture)
}

function Get-Number {
    param($Value)
    if ($null -eq $Value) { return 0 }
    try { return [double]$Value } catch { return 0 }
}

# What one API answer put in the context: the input it was sent, read from the
# cache or written to it. (Its own output is part of the next answer's input.)
function Get-UsageTokens {
    param($Usage)
    if ($null -eq $Usage) { return 0 }
    return (Get-Number $Usage.input_tokens) + (Get-Number $Usage.cache_creation_input_tokens) + (Get-Number $Usage.cache_read_input_tokens)
}

# The context used, from the newest main-thread answer in the transcript that
# carries a usage block (a side agent's answers are its own context, not this one).
function Get-TranscriptTokens {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    $lines = @(Get-Content -LiteralPath $Path -Tail 400 -Encoding UTF8 -ErrorAction Stop)
    for ($i = $lines.Count - 1; $i -ge 0; $i--) {
        $line = $lines[$i]
        if ($line -notmatch '"usage"' -or $line -notmatch '"assistant"') { continue }
        try { $entry = $line | ConvertFrom-Json } catch { continue }
        if ($entry.type -ne 'assistant' -or $entry.isSidechain -eq $true -or $entry.isApiErrorMessage -eq $true) { continue }
        $tokens = Get-UsageTokens $entry.message.usage
        if ($tokens -gt 0) { return $tokens }
    }
    return $null
}

$model = 'Claude'
$percent = '--'
$tokenText = '--'
try {
    $raw = [Console]::In.ReadToEnd()
    $j = $raw | ConvertFrom-Json
    if ($j.model.display_name) { $model = [string]$j.model.display_name }
    elseif ($j.model.id) { $model = [string]$j.model.id }

    $window = $j.context_window
    # the window's size: as Claude Code says it, else 1M for a [1m] model, else 200k
    $size = Get-Number $window.context_window_size
    if ($size -le 0) {
        $size = 200000
        if ("$($j.model.id) $($j.model.display_name)" -match '(?i)\[1m\]|1m context|\(1M') { $size = 1000000 }
    }

    $tokens = $null
    if ($null -ne $window -and $null -ne $window.current_usage) { $tokens = Get-UsageTokens $window.current_usage }
    if (-not $tokens) { $tokens = Get-TranscriptTokens ([string]$j.transcript_path) }

    if ($null -ne $window -and $null -ne $window.used_percentage) {
        $percent = [string][math]::Min(100, [math]::Round((Get-Number $window.used_percentage)))
        if (-not $tokens) { $tokens = [math]::Round((Get-Number $window.used_percentage) / 100 * $size) }
    } elseif ($tokens) {
        $percent = [string][math]::Min(100, [math]::Round($tokens / $size * 100))
    } else {
        # a fresh session: nothing has been sent yet
        $percent = '0'
        $tokens = 0
    }
    $tokenText = Get-TokenText $tokens
} catch {
    $percent = '--'
    $tokenText = '--'
}

[Console]::Out.WriteLine($model + ' | ' + $percent + '% | ' + $tokenText + ' tokens')
