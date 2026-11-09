# test-helpers.ps1 - offline checks for the Castle helper scripts.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File test-helpers.ps1
#
# Every helper runs in a child process with the four key variables removed,
# APPDATA pointed at a temp folder and CASTLE_NO_NETWORK=1, so nothing here
# can reach a paid API. Checks:
#   - no key            -> exit 3, fallback file written, one notices.jsonl line
#   - CASTLE_IMAGE=codex -> castle-image exits 4 and writes nothing
#   - -WhatIfCost with a fake key -> exit 0, no request, nothing written
#   - fake key, no -WhatIfCost -> the network guard fires (exit 9), proving the
#     guard sits in front of the request and the -WhatIfCost runs are honest
#   - Protect-Secret and Get-CastleKey behave
# Prints PASS/FAIL lines and exits 1 when anything fails.

$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$keyNames = @('OPENAI_API_KEY', 'ARK_API_KEY', 'SUNOAPI_ORG_KEY', 'ELEVENLABS_API_KEY')
$fakeKey = 'castle-test-fake-0123456789'

$root = Join-Path ([System.IO.Path]::GetTempPath()) ('castle-helpers-test-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$appData = Join-Path $root 'appdata'
$out = Join-Path $root 'out folder'
New-Item -ItemType Directory -Path $appData -Force | Out-Null
New-Item -ItemType Directory -Path $out -Force | Out-Null
$notices = Join-Path $appData 'MemoryCore Castle\notices.jsonl'

$script:failures = 0
function Report {
    param([bool]$Ok, [string]$Name, [string]$Detail = '')
    if ($Ok) { Write-Host "PASS  $Name" }
    else {
        $script:failures++
        Write-Host "FAIL  $Name"
        if ($Detail) { Write-Host ('      ' + ($Detail -replace "`r?`n", "`n      ")) }
    }
}

function Invoke-Helper {
    # Runs one helper in a fresh powershell.exe with a controlled environment.
    param([string]$Script, [string[]]$Arguments, [hashtable]$SetEnv = @{})
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $quoted = @()
    foreach ($a in $Arguments) {
        if ($a -match '^-[A-Za-z]+$') { $quoted += $a } else { $quoted += ('"' + $a + '"') }
    }
    $psi.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + (Join-Path $here $Script) + '" ' + ($quoted -join ' ')
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    foreach ($k in $keyNames + @('CASTLE_IMAGE', 'CASTLE_CLI')) {
        if ($psi.EnvironmentVariables.ContainsKey($k)) { $psi.EnvironmentVariables.Remove($k) }
    }
    $psi.EnvironmentVariables['APPDATA'] = $appData
    $psi.EnvironmentVariables['CASTLE_TOOLS'] = $here
    $psi.EnvironmentVariables['CASTLE_NO_NETWORK'] = '1'
    foreach ($k in $SetEnv.Keys) { $psi.EnvironmentVariables[$k] = [string]$SetEnv[$k] }
    $p = [System.Diagnostics.Process]::Start($psi)
    $so = $p.StandardOutput.ReadToEndAsync()
    $se = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit(120000)) { $p.Kill(); throw "$Script timed out" }
    $p.WaitForExit()
    return [pscustomobject]@{ Code = $p.ExitCode; Out = ($so.Result + $se.Result).Trim() }
}

function Get-NoticeCount {
    if (-not (Test-Path -LiteralPath $notices)) { return 0 }
    return @([System.IO.File]::ReadAllLines($notices) | Where-Object { $_.Trim() }).Count
}

function Get-LastNotice {
    $lines = @([System.IO.File]::ReadAllLines($notices) | Where-Object { $_.Trim() })
    return ($lines[-1] | ConvertFrom-Json)
}

$lyricsFile = Join-Path $root 'lyrics.md'
[System.IO.File]::WriteAllText($lyricsFile, "Verse one`r`nUnder the castle light`r`n", (New-Object System.Text.UTF8Encoding($false)))

$cases = @(
    @{ Kind = 'image'; Script = 'castle-image.ps1'; Key = 'OPENAI_API_KEY'; Target = (Join-Path $out 'pic.png'); Fallback = (Join-Path $out 'pic.prompt.md'); Needle = 'a lantern in the rain'
       Args = @('-Prompt', 'a lantern in the rain', '-OutFile', (Join-Path $out 'pic.png')); Env = @{ CASTLE_IMAGE = 'openai' } }
    @{ Kind = 'video'; Script = 'castle-video.ps1'; Key = 'ARK_API_KEY'; Target = (Join-Path $out 'clip.mp4'); Fallback = (Join-Path $out 'clip.prompt.md'); Needle = 'waves at dusk'
       Args = @('-Prompt', 'waves at dusk', '-OutFile', (Join-Path $out 'clip.mp4')); Env = @{} }
    @{ Kind = 'song'; Script = 'castle-song.ps1'; Key = 'SUNOAPI_ORG_KEY'; Target = (Join-Path $out 'songs\01-castle-light.mp3'); Fallback = (Join-Path $out 'songs\castle-light.lyrics.md'); Needle = 'Under the castle light'
       Args = @('-Title', 'Castle Light', '-Style', 'soft piano, warm vocal', '-Lyrics', $lyricsFile, '-OutFolder', (Join-Path $out 'songs')); Env = @{} }
    @{ Kind = 'voice'; Script = 'castle-voice.ps1'; Key = 'ELEVENLABS_API_KEY'; Target = (Join-Path $out 'line.mp3'); Fallback = (Join-Path $out 'line.line.txt'); Needle = 'Good evening, welcome back.'
       Args = @('-Text', 'Good evening, welcome back.', '-OutFile', (Join-Path $out 'line.mp3')); Env = @{} }
)

Write-Host "Temp folder: $root"
Write-Host ''

# 1) No key -> fallback, exit 3, notice line.
foreach ($c in $cases) {
    $before = Get-NoticeCount
    $r = Invoke-Helper -Script $c.Script -Arguments $c.Args -SetEnv $c.Env
    Report ($r.Code -eq 3) "$($c.Kind): no key exits 3" "exit $($r.Code): $($r.Out)"
    $fbOk = (Test-Path -LiteralPath $c.Fallback)
    $fbText = if ($fbOk) { [System.IO.File]::ReadAllText($c.Fallback, [System.Text.Encoding]::UTF8) } else { '' }
    Report ($fbOk -and $fbText.Contains('Saved instead of rendering:') -and $fbText.Contains("Castle's Settings") -and $fbText.Contains($c.Needle)) "$($c.Kind): fallback file holds header and content" "file: $($c.Fallback)"
    Report (-not (Test-Path -LiteralPath $c.Target)) "$($c.Kind): no rendered file was written"
    $after = Get-NoticeCount
    $noticeOk = $false
    $noticeDetail = "lines before $before, after $after"
    if ($after -eq $before + 1) {
        try {
            $n = Get-LastNotice
            $noticeOk = ($n.kind -eq $c.Kind) -and ($n.settings -eq $c.Key) -and ($n.file -eq $c.Fallback) -and $n.at -and $n.reason
            $noticeDetail = ($n | ConvertTo-Json -Compress)
        } catch { $noticeDetail = $_.Exception.Message }
    }
    Report $noticeOk "$($c.Kind): one notices.jsonl line (kind, file, settings)" $noticeDetail
    Report ($r.Out -notmatch 'castle-test-fake') "$($c.Kind): no key text in output"
}

# 2) Codex draws its own pictures -> exit 4, nothing written.
$codexOut = Join-Path $out 'codex.png'
$before = Get-NoticeCount
$r = Invoke-Helper -Script 'castle-image.ps1' -Arguments @('-Prompt', 'a fox', '-OutFile', $codexOut) -SetEnv @{ CASTLE_IMAGE = 'codex'; CASTLE_CLI = 'codex'; OPENAI_API_KEY = $fakeKey }
Report ($r.Code -eq 4 -and $r.Out -match 'Codex draws pictures with its own image tool') 'image: CASTLE_IMAGE=codex exits 4 with the hand-off line' "exit $($r.Code): $($r.Out)"
Report (-not (Test-Path -LiteralPath $codexOut) -and -not (Test-Path -LiteralPath (Join-Path $out 'codex.prompt.md')) -and (Get-NoticeCount) -eq $before) 'image: codex hand-off writes no file and no notice'

# 3) -WhatIfCost with a fake key -> exit 0, estimate printed, no request.
foreach ($c in $cases) {
    $envs = @{}; foreach ($k in $c.Env.Keys) { $envs[$k] = $c.Env[$k] }
    $envs[$c.Key] = $fakeKey
    $before = Get-NoticeCount
    $r = Invoke-Helper -Script $c.Script -Arguments ($c.Args + @('-WhatIfCost')) -SetEnv $envs
    Report ($r.Code -eq 0 -and $r.Out -match 'cost_estimate_usd: \d' -and $r.Out -notmatch 'CASTLE_NO_NETWORK') "$($c.Kind): -WhatIfCost exits 0 before any request" "exit $($r.Code): $($r.Out)"
    Report (-not (Test-Path -LiteralPath $c.Target) -and (Get-NoticeCount) -eq $before -and $r.Out -notmatch 'castle-test-fake') "$($c.Kind): -WhatIfCost writes nothing and shows no key"
}

# 4) Fake key without -WhatIfCost -> the network guard is the first thing hit.
foreach ($c in $cases) {
    $envs = @{}; foreach ($k in $c.Env.Keys) { $envs[$k] = $c.Env[$k] }
    $envs[$c.Key] = $fakeKey
    $r = Invoke-Helper -Script $c.Script -Arguments $c.Args -SetEnv $envs
    Report ($r.Code -eq 9 -and $r.Out -match 'CASTLE_NO_NETWORK: blocked' -and $r.Out -notmatch 'castle-test-fake') "$($c.Kind): with a key the request path is guarded (exit 9)" "exit $($r.Code): $($r.Out)"
}

# 5) Image edit path with a missing reference -> exit 2 (bad input), no request.
$r = Invoke-Helper -Script 'castle-image.ps1' -Arguments @('-Prompt', 'restyle', '-OutFile', (Join-Path $out 'ref.png'), '-Reference', (Join-Path $out 'missing.png')) -SetEnv @{ CASTLE_IMAGE = 'openai'; OPENAI_API_KEY = $fakeKey }
Report ($r.Code -eq 2) 'image: missing -Reference file exits 2' "exit $($r.Code): $($r.Out)"

# 6) The world prepare helper: a missing piece is named, and no Node leaves a notice.
$worldDir = Join-Path $root 'world'
$hd = Join-Path $worldDir 'hd'
New-Item -ItemType Directory -Path $hd -Force | Out-Null
$manifest = Join-Path $worldDir 'world.json'
[System.IO.File]::WriteAllText($manifest, '{"slug":"t","pieces":[{"file":"01-base.glb","role":"base","size":40},{"file":"02-crystal.glb","role":"companion","size":9}]}', (New-Object System.Text.UTF8Encoding($false)))
[System.IO.File]::WriteAllBytes((Join-Path $hd '01 my base export.glb'), [byte[]](0x67, 0x6C, 0x54, 0x46))
$r = Invoke-Helper -Script 'castle-world.ps1' -Arguments @('-Prepare', '-Manifest', $manifest, '-In', $hd, '-Out', (Join-Path $worldDir 'pieces'))
Report ($r.Code -eq 2 -and $r.Out -match '02-crystal\.glb' -and $r.Out -notmatch '01-base') 'world: a missing piece is named (exit 2)' "exit $($r.Code): $($r.Out)"
[System.IO.File]::WriteAllBytes((Join-Path $hd '2.glb'), [byte[]](0x67, 0x6C, 0x54, 0x46))
$before = Get-NoticeCount
$r = Invoke-Helper -Script 'castle-world.ps1' -Arguments @('-Prepare', '-Manifest', $manifest, '-In', $hd, '-Out', (Join-Path $worldDir 'pieces')) -SetEnv @{ PATH = (Join-Path $env:SystemRoot 'System32') }
$noticeOk = ((Get-NoticeCount) -eq $before + 1) -and ((Get-LastNotice).kind -eq 'world')
Report ($r.Code -eq 3 -and $r.Out -match 'Node is not installed' -and $noticeOk) 'world: no Node exits 3 with a notice' "exit $($r.Code): $($r.Out)"
Report (-not (Test-Path -LiteralPath (Join-Path $worldDir 'pieces'))) 'world: nothing written without Node'

# 7) Unit checks on the shared functions.
. (Join-Path $here 'castle-common.ps1')
$leak = Protect-Secret 'HTTP 401 - Incorrect API key provided: sk-proj-AbCdEf1234567890XyZ. Authorization: Bearer abc.def.ghi xi-api-key: 0123456789abcdef'
Report ($leak -notmatch 'sk-proj-AbCd' -and $leak -notmatch 'abc\.def\.ghi' -and $leak -notmatch '0123456789abcdef' -and $leak -match 'HTTP 401') 'common: Protect-Secret strips key-like text' $leak
$long = Protect-Secret ('token ' + ('a1B2' * 12))
Report ($long -notmatch 'a1B2a1B2') 'common: Protect-Secret strips long tokens' $long

$saved = [Environment]::GetEnvironmentVariable('CASTLE_TEST_KEY', 'Process')
$okKey = $true
foreach ($v in @('', '   ', 'your-key-here', '<paste key>', 'sk-...', 'changeme', 'xxxxxxxxxxxx', 'PLACEHOLDER')) {
    [Environment]::SetEnvironmentVariable('CASTLE_TEST_KEY', $v, 'Process')
    if ($null -ne (Get-CastleKey 'CASTLE_TEST_KEY')) { $okKey = $false; Write-Host "      treated as a key: '$v'" }
}
[Environment]::SetEnvironmentVariable('CASTLE_TEST_KEY', $fakeKey, 'Process')
if ((Get-CastleKey 'CASTLE_TEST_KEY') -ne $fakeKey) { $okKey = $false; Write-Host '      a real-looking key was refused' }
[Environment]::SetEnvironmentVariable('CASTLE_TEST_KEY', $saved, 'Process')
Report $okKey 'common: Get-CastleKey treats empty and placeholder values as missing'

Write-Host ''
try { Remove-Item -LiteralPath $root -Recurse -Force } catch {}
if ($script:failures -gt 0) {
    Write-Host "$($script:failures) check(s) FAILED"
    exit 1
}
Write-Host 'All checks passed.'
exit 0
