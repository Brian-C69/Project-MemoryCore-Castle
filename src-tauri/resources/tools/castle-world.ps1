# castle-world.ps1 - prepare a world's model pieces for Castle.
#
#   castle-world.ps1 -Prepare -Manifest <world.json> -In <folder of exports> -Out <pieces folder>
#
# The member makes each piece in Tripo (or any 3D tool) and exports a GLB named like its
# picture's number ("01-....glb" or just "01.glb"). This makes every piece light enough for the
# world windows: shapes simplified to a budget (the base 60k triangles, every other piece 20k),
# textures turned to webp at most 1024 px, everything meshopt-compressed - with the free
# gltf-transform tool through npx, which comes with Node (Claude Code and Codex bring Node).
#
# Windows PowerShell 5.1, ASCII-only source. Exit codes:
#   0  every piece prepared
#   1  the tool failed on a piece (one line says which and why)
#   2  bad input (no manifest, or pieces missing from the exports folder - listed)
#   3  Node is not installed - a notice was left for Castle
param(
    [switch]$Prepare,
    [Parameter(Mandatory = $true)][string]$Manifest,
    [Parameter(Mandatory = $true)][string]$In,
    [Parameter(Mandatory = $true)][string]$Out
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'castle-common.ps1')

$GltfTransform = '@gltf-transform/cli@4'
$Budgets = @{ base = 60000; default = 20000 }

function Get-GlbTriangles {
    # Counts the triangles a GLB draws, from its JSON chunk alone.
    param([string]$Path)
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 20 -or [BitConverter]::ToUInt32($bytes, 0) -ne 0x46546C67) { return 0 }
    $jsonLength = [BitConverter]::ToUInt32($bytes, 12)
    $gltf = [System.Text.Encoding]::UTF8.GetString($bytes, 20, $jsonLength) | ConvertFrom-Json
    $total = 0
    foreach ($mesh in @($gltf.meshes)) {
        foreach ($p in @($mesh.primitives)) {
            if ($null -ne $p.mode -and $p.mode -ne 4) { continue }
            if ($null -ne $p.indices) { $total += [int]$gltf.accessors[[int]$p.indices].count / 3 }
            elseif ($null -ne $p.attributes.POSITION) { $total += [int]$gltf.accessors[[int]$p.attributes.POSITION].count / 3 }
        }
    }
    return [int]$total
}

function Write-WorldNotice {
    param([string]$Reason, [string]$File)
    $appData = $env:APPDATA
    if ([string]::IsNullOrWhiteSpace($appData)) { $appData = [Environment]::GetFolderPath('ApplicationData') }
    $noticeDir = Join-Path $appData 'MemoryCore Castle'
    if (-not (Test-Path -LiteralPath $noticeDir)) { New-Item -ItemType Directory -Path $noticeDir -Force | Out-Null }
    $notice = [ordered]@{ at = (Get-CastleNow); kind = 'world'; file = $File; reason = $Reason; settings = '' }
    [System.IO.File]::AppendAllText((Join-Path $noticeDir 'notices.jsonl'), (($notice | ConvertTo-Json -Compress) + "`n"), $script:CastleUtf8)
}

if (-not $Prepare) { Stop-CastleBadInput 'Say -Prepare to prepare the pieces.' }
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) { Stop-CastleBadInput "No world.json at $Manifest." }
if (-not (Test-Path -LiteralPath $In -PathType Container)) { Stop-CastleBadInput "No exports folder at $In." }

$world = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $Manifest).ProviderPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
$pieces = @($world.pieces | Where-Object { $_.file -match '^\d+' })
if (-not $pieces.Count) { Stop-CastleBadInput 'The world.json lists no numbered pieces.' }

# every piece needs its export: matched by its number
$exports = @(Get-ChildItem -LiteralPath $In -Filter '*.glb' -File)
$plan = @()
$missing = @()
foreach ($piece in $pieces) {
    $number = [int]([regex]::Match($piece.file, '^\d+').Value)
    $found = $exports | Where-Object { $_.BaseName -match '^0*(\d+)(\D|$)' -and [int]$Matches[1] -eq $number } | Select-Object -First 1
    if ($found) { $plan += [pscustomobject]@{ Piece = $piece; Source = $found.FullName } }
    else { $missing += $piece.file }
}
if ($missing.Count) { Stop-CastleBadInput ("These pieces are not in the exports folder yet: " + ($missing -join ', ')) }

# the tool comes with Node
$npx = Get-Command 'npx.cmd' -ErrorAction SilentlyContinue
if (-not $npx) { $npx = Get-Command 'npx' -ErrorAction SilentlyContinue }
if (-not $npx) {
    Write-WorldNotice -Reason 'Node is not installed, so the world pieces could not be prepared. Install Node from nodejs.org, then press Prepare pieces again.' -File $In
    Write-CastleLine 'Node is not installed (npx was not found), so the pieces could not be prepared. Install Node from nodejs.org and try again.'
    exit 3
}
Assert-CastleNetworkAllowed -Url 'npx (the gltf-transform tool may be downloaded once)'

New-Item -ItemType Directory -Path $Out -Force | Out-Null
$done = 0
foreach ($step in $plan) {
    $piece = $step.Piece
    $target = Join-Path $Out $piece.file
    $budget = if ($piece.role -eq 'base') { $Budgets.base } else { $Budgets.default }
    $before = Get-GlbTriangles -Path $step.Source
    $ratio = if ($before -gt $budget) { [Math]::Round($budget / $before, 4) } else { 1 }
    $toolArgs = @('--yes', $GltfTransform, 'optimize', $step.Source, $target, '--compress', 'meshopt', '--texture-compress', 'webp', '--texture-size', '1024')
    if ($ratio -lt 1) { $toolArgs += @('--simplify', 'true', '--simplify-ratio', $ratio.ToString([System.Globalization.CultureInfo]::InvariantCulture), '--simplify-error', '0.01') }
    else { $toolArgs += @('--simplify', 'false') }
    $sizeBefore = (Get-Item -LiteralPath $step.Source).Length
    # npm prints warnings on stderr; Windows PowerShell would treat them as failures under Stop
    $ErrorActionPreference = 'Continue'
    $output = & $npx.Source @toolArgs 2>&1 | Out-String
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $target)) {
        $why = ($output -split "`r?`n" | Where-Object { $_ -match '\S' } | Select-Object -Last 2) -join ' '
        Stop-CastleFailure -What "Preparing $($piece.file)" -Message $why
    }
    $after = Get-GlbTriangles -Path $target
    $sizeAfter = (Get-Item -LiteralPath $target).Length
    $done++
    Write-CastleLine ("piece {0}: {1:N1} MB -> {2:N1} MB, {3} -> {4} triangles ({5}/{6})" -f $piece.file, ($sizeBefore / 1MB), ($sizeAfter / 1MB), $before, $after, $done, $plan.Count)
}
Write-CastleLine "result: prepared $done pieces into $Out"
exit 0
