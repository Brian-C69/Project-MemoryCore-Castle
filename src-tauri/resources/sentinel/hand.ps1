# The Sentinel's hand: the one helper with admin rights. It lives where only an administrator can
# change it, uses the system's own commands only, and trusts nothing it is handed: every request
# is checked again here, and a move or an argument that is not on its lists is refused.
#
# The hand cannot tell who wrote a request, so every move on its lists only ever closes the
# machine further. Which moves the companion may ask for alone, and which need the member's
# click, is decided by Castle before a request is written; here both lists are allowed.
#
# Requests: <queue>\<id>.json  {"id":"123","move":"firewall-on","arg":""}
# Answers:  <done>\<id>.json   {"id","move","arg","ok","result","undo","events","at","dry"}
# Windows PowerShell 5.1. ASCII only.
param(
    [switch]$DryRun,
    [string]$Queue = '',
    [string]$Done = ''
)

$ErrorActionPreference = 'Stop'
$Group = 'MemoryCore Castle Sentinel'
$RulePrefix = 'MCSentinel-'
$Root = Join-Path $env:ProgramData 'MemoryCore Castle Sentinel'
if (-not $DryRun -or $Queue -eq '') { $Queue = Join-Path $Root 'queue' }
if (-not $DryRun -or $Done -eq '') { $Done = Join-Path $Root 'done' }

$Alone = @('firewall-on', 'protection-on', 'scan-quick', 'signatures-update', 'block-address')
$Click = @('block-port', 'disable-task', 'disable-startup', 'stop-process')
$Reads = @('signins', 'undo')

# Never stopped, whoever asks.
$Protected = @('system', 'idle', 'registry', 'secure system', 'memory compression', 'smss', 'csrss', 'wininit',
    'winlogon', 'services', 'lsass', 'lsaiso', 'svchost', 'msmpeng', 'nissrv', 'mpdefendercoreservice', 'securityhealthservice',
    'dwm', 'fontdrvhost', 'explorer', 'sihost', 'ctfmon', 'powershell', 'taskhostw', 'spoolsv', 'audiodg', 'conhost')

$StartupKeys = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder'
)

function Is-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    return (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Answer($id, $move, $arg, $ok, $result, $undo, $events) {
    $answer = [ordered]@{
        id = $id; move = $move; arg = $arg; ok = [bool]$ok; result = [string]$result
        undo = $undo; events = $events; at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'); dry = [bool]$DryRun
    }
    $path = Join-Path $Done ($id + '.json')
    $temp = $path + '.partial'
    [IO.File]::WriteAllText($temp, ($answer | ConvertTo-Json -Depth 6 -Compress), (New-Object Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $temp -Destination $path -Force
}

# --- the checks: everything is refused unless it is plainly safe ---

function Own-Addresses {
    $own = @()
    try { foreach ($a in Get-NetIPAddress) { $own += ([string]$a.IPAddress).Split('%')[0].ToLower() } } catch { }
    try {
        foreach ($r in Get-NetRoute -DestinationPrefix '0.0.0.0/0', '::/0' -ErrorAction SilentlyContinue) {
            $own += ([string]$r.NextHop).Split('%')[0].ToLower()
        }
    } catch { }
    return $own
}

function Address-Problem([string]$text) {
    $ip = $null
    if ($text -notmatch '^[0-9A-Fa-f:.]{2,45}$' -or -not [Net.IPAddress]::TryParse($text, [ref]$ip)) { return 'not an address' }
    if ([Net.IPAddress]::IsLoopback($ip)) { return 'the loopback' }
    $bytes = $ip.GetAddressBytes()
    if ($ip.AddressFamily -eq 'InterNetwork') {
        if ($bytes[0] -eq 0 -or $bytes[0] -ge 224) { return 'a reserved address' }
        if ($bytes[0] -eq 169 -and $bytes[1] -eq 254) { return 'a link-local address' }
    } else {
        if ($ip.Equals([Net.IPAddress]::IPv6Any) -or $ip.IsIPv6LinkLocal -or $ip.IsIPv6Multicast -or $ip.IsIPv6SiteLocal) { return 'a reserved address' }
        if ($bytes[0] -eq 0) { return 'a reserved address' }
    }
    $plain = $ip.ToString().ToLower()
    if ((Own-Addresses) -contains $plain) { return "this computer's own address or its router" }
    return ''
}

function Rule-Name([string]$kind, [string]$value) {
    return $RulePrefix + $kind + '-' + ($value -replace '[^0-9A-Za-z]', '_')
}

# --- the moves ---

function Remove-ExpiredBlocks {
    if ($DryRun) { return }
    try {
        foreach ($rule in Get-NetFirewallRule -Group $Group -ErrorAction SilentlyContinue) {
            if ([string]$rule.Description -match '^expires (\d{4}-\d{2}-\d{2} \d{2}:\d{2})$') {
                $until = [datetime]::ParseExact($Matches[1], 'yyyy-MM-dd HH:mm', $null)
                if ($until -lt (Get-Date)) { Remove-NetFirewallRule -Name $rule.Name }
            }
        }
    } catch { }
}

function Move-FirewallOn {
    if ($DryRun) { return @($true, 'dry: would switch every firewall profile on', $null) }
    Set-NetFirewallProfile -All -Enabled True
    return @($true, 'Every firewall profile is on.', $null)
}

function Move-ProtectionOn {
    if ($DryRun) { return @($true, 'dry: would switch real-time protection on', $null) }
    Set-MpPreference -DisableRealtimeMonitoring $false
    return @($true, 'Real-time protection is on.', $null)
}

function Move-ScanQuick {
    $tool = Join-Path $env:ProgramFiles 'Windows Defender\MpCmdRun.exe'
    if (-not (Test-Path -LiteralPath $tool)) { return @($false, 'The antivirus scanner was not found.', $null) }
    if ($DryRun) { return @($true, 'dry: would start a quick scan', $null) }
    Start-Process -FilePath $tool -ArgumentList '-Scan', '-ScanType', '1' -WindowStyle Hidden | Out-Null
    return @($true, 'A quick scan has started.', $null)
}

function Move-SignaturesUpdate {
    if ($DryRun) { return @($true, 'dry: would update the antivirus signatures', $null) }
    Update-MpSignature
    return @($true, 'The antivirus signatures are updated.', $null)
}

function Move-BlockAddress([string]$arg) {
    $problem = Address-Problem $arg
    if ($problem -ne '') { return @($false, "Refused: $problem.", $null) }
    $name = Rule-Name 'address' $arg
    $until = (Get-Date).AddHours(24).ToString('yyyy-MM-dd HH:mm')
    $undo = [ordered]@{ kind = 'remove-rule'; name = $name }
    if ($DryRun) { return @($true, "dry: would block $arg until $until", $undo) }
    $existing = Get-NetFirewallRule -Name $name -ErrorAction SilentlyContinue
    if ($existing) {
        Set-NetFirewallRule -Name $name -Description ('expires ' + $until)
    } else {
        New-NetFirewallRule -Name $name -DisplayName ("$Group - block $arg") -Group $Group -Direction Inbound -Action Block -RemoteAddress $arg -Description ('expires ' + $until) | Out-Null
    }
    return @($true, "$arg is blocked until $until.", $undo)
}

function Move-BlockPort([string]$arg) {
    if ($arg -notmatch '^\d{1,5}$' -or [int]$arg -lt 1 -or [int]$arg -gt 65535) { return @($false, 'Refused: not a port.', $null) }
    $name = Rule-Name 'port' $arg
    $undo = [ordered]@{ kind = 'remove-rule'; name = $name }
    if ($DryRun) { return @($true, "dry: would block incoming connections to port $arg", $undo) }
    if (-not (Get-NetFirewallRule -Name $name -ErrorAction SilentlyContinue)) {
        New-NetFirewallRule -Name $name -DisplayName ("$Group - block port $arg") -Group $Group -Direction Inbound -Action Block -Protocol TCP -LocalPort ([int]$arg) | Out-Null
    }
    return @($true, "Incoming connections to port $arg are blocked.", $undo)
}

function Split-TaskPath([string]$arg) {
    if ($arg.Length -gt 260 -or $arg -notmatch '^\\' -or $arg.Contains('..') -or $arg -match '[\x00-\x1f*?"<>|]') { return $null }
    $cut = $arg.LastIndexOf('\')
    $path = $arg.Substring(0, $cut + 1)
    $name = $arg.Substring($cut + 1)
    if ($name -eq '') { return $null }
    return @($path, $name)
}

function Move-DisableTask([string]$arg) {
    $parts = Split-TaskPath $arg
    if ($null -eq $parts) { return @($false, 'Refused: not a task name.', $null) }
    if ($parts[0] -like '\Microsoft\*' -or $parts[0] -like ('\' + $Group + '\*')) { return @($false, "Refused: Windows' own tasks and the Sentinel's tasks are never switched off.", $null) }
    $task = Get-ScheduledTask -TaskPath $parts[0] -TaskName $parts[1] -ErrorAction SilentlyContinue
    if (-not $task) { return @($false, 'That task was not found.', $null) }
    $undo = [ordered]@{ kind = 'enable-task'; path = $parts[0]; name = $parts[1] }
    if ($DryRun) { return @($true, "dry: would switch off the task $arg", $undo) }
    Disable-ScheduledTask -TaskPath $parts[0] -TaskName $parts[1] | Out-Null
    return @($true, "The task $arg is switched off.", $undo)
}

function Move-DisableStartup([string]$arg) {
    if ($arg.Length -lt 1 -or $arg.Length -gt 200 -or $arg -match '[\x00-\x1f\\/]') { return @($false, 'Refused: not a startup entry name.', $null) }
    $runKeys = @(
        @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Run', 0),
        @('HKCU:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Run', 1),
        @('HKLM:\Software\Microsoft\Windows\CurrentVersion\Run', 3),
        @('HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Run', 4)
    )
    $target = $null
    foreach ($pair in $runKeys) {
        $values = Get-ItemProperty -LiteralPath $pair[0] -ErrorAction SilentlyContinue
        if ($values -and ($values.PSObject.Properties.Name -contains $arg)) { $target = @($StartupKeys[$pair[1]], $arg); break }
    }
    if ($null -eq $target) {
        $folders = @(@([Environment]::GetFolderPath('Startup'), 2), @([Environment]::GetFolderPath('CommonStartup'), 5))
        foreach ($pair in $folders) {
            foreach ($file in @($arg, ($arg + '.lnk'))) {
                if ($pair[0] -and (Test-Path -LiteralPath (Join-Path $pair[0] $file))) { $target = @($StartupKeys[$pair[1]], $file); break }
            }
            if ($target) { break }
        }
    }
    if ($null -eq $target) { return @($false, 'That startup entry was not found.', $null) }
    $old = $null
    $current = Get-ItemProperty -LiteralPath $target[0] -Name $target[1] -ErrorAction SilentlyContinue
    if ($current) { $old = [Convert]::ToBase64String([byte[]]$current.($target[1])) }
    $undo = [ordered]@{ kind = 'restore-startup'; key = $target[0]; value = $target[1]; old = $old }
    if ($DryRun) { return @($true, "dry: would switch off the startup entry $arg", $undo) }
    if (-not (Test-Path -LiteralPath $target[0])) { New-Item -Path $target[0] -Force | Out-Null }
    $off = [byte[]](3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    New-ItemProperty -LiteralPath $target[0] -Name $target[1] -PropertyType Binary -Value $off -Force | Out-Null
    return @($true, "The startup entry $arg is switched off.", $undo)
}

function Move-StopProcess([string]$arg) {
    if ($arg -notmatch '^\d{1,7}$' -or [int]$arg -lt 8 -or [int]$arg -eq $PID) { return @($false, 'Refused: not a process this hand may stop.', $null) }
    $proc = Get-Process -Id ([int]$arg) -ErrorAction SilentlyContinue
    if (-not $proc) { return @($false, 'That program is not running.', $null) }
    if ($Protected -contains $proc.ProcessName.ToLower()) { return @($false, 'Refused: a part of Windows or its protection is never stopped.', $null) }
    if ($DryRun) { return @($true, "dry: would stop $($proc.ProcessName) ($arg)", $null) }
    Stop-Process -Id ([int]$arg) -Force
    return @($true, "$($proc.ProcessName) ($arg) is stopped.", $null)
}

function Read-SignIns {
    $events = @()
    try {
        $found = Get-WinEvent -FilterHashtable @{ LogName = 'Security'; Id = 4625, 4624, 4720, 4732; StartTime = (Get-Date).AddHours(-24) } -MaxEvents 2000 -ErrorAction Stop
        foreach ($e in $found) {
            $data = @{}
            foreach ($d in ([xml]$e.ToXml()).Event.EventData.Data) { $data[[string]$d.Name] = [string]$d.'#text' }
            if ($e.Id -eq 4624 -and $data['LogonType'] -ne '10') { continue }
            $events += [ordered]@{
                id = $e.Id; at = $e.TimeCreated.ToString('yyyy-MM-dd HH:mm:ss')
                user = $data['TargetUserName']; address = $data['IpAddress']; member = $data['MemberName']; by = $data['SubjectUserName']
            }
            if ($events.Count -ge 500) { break }
        }
    } catch {
        if ($_.FullyQualifiedErrorId -notlike 'NoMatchingEventsFound*') { return @($false, 'The sign-in log could not be read.', $null, @()) }
    }
    return @($true, "$($events.Count) sign-in event(s) in the last day.", $null, $events)
}

function Undo-Move([string]$arg) {
    if ($arg -notmatch '^\d{1,9}$') { return @($false, 'Refused: not a request number.', $null) }
    $path = Join-Path $Done ($arg + '.json')
    $mark = Join-Path $Done ($arg + '.undone')
    if (-not (Test-Path -LiteralPath $path)) { return @($false, 'That move was not found.', $null) }
    if (Test-Path -LiteralPath $mark) { return @($false, 'That move was already undone.', $null) }
    $earlier = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    $recipe = $earlier.undo
    if ($null -eq $recipe -or -not $earlier.ok -or $earlier.dry) { return @($false, 'That move has nothing to undo.', $null) }
    switch ([string]$recipe.kind) {
        'remove-rule' {
            if (-not ([string]$recipe.name).StartsWith($RulePrefix)) { return @($false, 'Refused: not one of the Sentinel rules.', $null) }
            if (-not $DryRun) { Remove-NetFirewallRule -Name $recipe.name -ErrorAction SilentlyContinue }
        }
        'enable-task' {
            if (-not $DryRun) { Enable-ScheduledTask -TaskPath $recipe.path -TaskName $recipe.name | Out-Null }
        }
        'restore-startup' {
            if ($StartupKeys -notcontains [string]$recipe.key) { return @($false, 'Refused: not a startup setting.', $null) }
            if (-not $DryRun) {
                if ($null -eq $recipe.old) {
                    Remove-ItemProperty -LiteralPath $recipe.key -Name $recipe.value -ErrorAction SilentlyContinue
                } else {
                    New-ItemProperty -LiteralPath $recipe.key -Name $recipe.value -PropertyType Binary -Value ([Convert]::FromBase64String($recipe.old)) -Force | Out-Null
                }
            }
        }
        default { return @($false, 'That move has nothing to undo.', $null) }
    }
    if (-not $DryRun) { Set-Content -LiteralPath $mark -Value 'undone' }
    return @($true, 'Undone.', $null)
}

# --- one run: the queue, at most twenty small requests ---

if (-not $DryRun -and -not (Is-Admin)) {
    Write-Output 'The hand needs administrator rights.'
    exit 5
}
if (-not (Test-Path -LiteralPath $Queue) -or -not (Test-Path -LiteralPath $Done)) {
    Write-Output 'The Sentinel is not installed.'
    exit 4
}

Remove-ExpiredBlocks

# answers older than a week are swept
try {
    Get-ChildItem -LiteralPath $Done -File | Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-7) } | Remove-Item -Force -ErrorAction SilentlyContinue
} catch { }

$files = @(Get-ChildItem -LiteralPath $Queue -File -Filter '*.json' | Sort-Object LastWriteTime | Select-Object -First 20)
foreach ($file in $files) {
    if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { continue }
    if ($file.Length -gt 4096) { Remove-Item -LiteralPath $file.FullName -Force; continue }
    $text = ''
    try { $text = [IO.File]::ReadAllText($file.FullName) } catch { continue }
    Remove-Item -LiteralPath $file.FullName -Force -ErrorAction SilentlyContinue
    $request = $null
    try { $request = $text | ConvertFrom-Json } catch { continue }
    $id = [string]$request.id
    if ($id -notmatch '^\d{1,9}$') { continue }
    $move = [string]$request.move
    $arg = [string]$request.arg
    if ($null -eq $request.arg) { $arg = '' }
    if (($Alone + $Click + $Reads) -notcontains $move) {
        Answer $id $move $arg $false 'Refused: that move is not on the lists.' $null $null
        continue
    }
    $noArg = @('firewall-on', 'protection-on', 'scan-quick', 'signatures-update', 'signins')
    if ($noArg -contains $move -and $arg -ne '') {
        Answer $id $move $arg $false 'Refused: this move takes nothing.' $null $null
        continue
    }
    try {
        $events = $null
        switch ($move) {
            'firewall-on' { $r = Move-FirewallOn }
            'protection-on' { $r = Move-ProtectionOn }
            'scan-quick' { $r = Move-ScanQuick }
            'signatures-update' { $r = Move-SignaturesUpdate }
            'block-address' { $r = Move-BlockAddress $arg }
            'block-port' { $r = Move-BlockPort $arg }
            'disable-task' { $r = Move-DisableTask $arg }
            'disable-startup' { $r = Move-DisableStartup $arg }
            'stop-process' { $r = Move-StopProcess $arg }
            'signins' { $r = Read-SignIns; $events = $r[3] }
            'undo' { $r = Undo-Move $arg }
        }
        Answer $id $move $arg $r[0] $r[1] $r[2] $events
    } catch {
        Answer $id $move $arg $false ('It did not work: ' + $_.Exception.Message) $null $null
    }
}
exit 0
