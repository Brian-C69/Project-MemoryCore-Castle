# The Sentinel's installer. The member runs it once, in a PowerShell opened as administrator, with
# the line MemoryCore Castle shows them; Castle never runs it and never asks for admin rights itself.
#
# It copies the hand to a folder only an administrator can change, makes the hand's queue (the
# member may write requests) and its answers (the member may only read them), and registers three
# scheduled tasks under \MemoryCore Castle Sentinel\:
#   Tick  - Castle's watch (--sentinel-tick) every five minutes, as the member, limited rights
#   Alarm - the same watch when the antivirus or the firewall writes an alarm event
#   Hand  - the hand, on demand, as the member with administrator rights
# -Uninstall removes all of it, and every firewall rule the Sentinel made.
# Windows PowerShell 5.1. ASCII only.
param(
    [string]$Castle = '',
    [string]$Hand = '',
    [string]$User = '',
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
$Name = 'MemoryCore Castle Sentinel'
$Program = Join-Path $env:ProgramFiles $Name
$Root = Join-Path $env:ProgramData $Name
$TaskPath = '\' + $Name + '\'

function Say([string]$line) { Write-Output ('SENTINEL  ' + $line) }

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Say 'Please open PowerShell as administrator (right-click, Run as administrator) and run the line again.'
    exit 5
}

if ($Uninstall) {
    foreach ($task in @('Tick', 'Alarm', 'Hand')) {
        Unregister-ScheduledTask -TaskPath $TaskPath -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    }
    try {
        $service = New-Object -ComObject 'Schedule.Service'
        $service.Connect()
        $service.GetFolder('\').DeleteFolder($Name, 0)
    } catch { }
    Get-NetFirewallRule -Group $Name -ErrorAction SilentlyContinue | Remove-NetFirewallRule -ErrorAction SilentlyContinue
    foreach ($folder in @($Program, $Root)) {
        if (Test-Path -LiteralPath $folder) { Remove-Item -LiteralPath $folder -Recurse -Force }
    }
    Say 'removed - the tasks, the hand, its folders and its firewall rules are gone.'
    exit 0
}

# --- what the line carries ---
if ($Castle -eq '' -or -not (Test-Path -LiteralPath $Castle -PathType Leaf) -or [IO.Path]::GetExtension($Castle) -ne '.exe') {
    Say 'The line names no MemoryCore Castle program. Copy the line from Castle again.'
    exit 2
}
if ($Hand -eq '' -or -not (Test-Path -LiteralPath $Hand -PathType Leaf)) {
    Say 'The line names no hand. Copy the line from Castle again.'
    exit 2
}
$handBytes = [IO.File]::ReadAllBytes($Hand)
foreach ($b in $handBytes) {
    if ($b -gt 127) { Say 'The hand is not the one Castle ships. Copy the line from Castle again.'; exit 2 }
}
if ($User -eq '') { $User = [Security.Principal.WindowsIdentity]::GetCurrent().Name }
try {
    $userSid = (New-Object Security.Principal.NTAccount($User)).Translate([Security.Principal.SecurityIdentifier]).Value
} catch {
    Say "The account $User was not found."
    exit 2
}

# --- the hand, where only an administrator can change it ---
New-Item -ItemType Directory -Path $Program -Force | Out-Null
$installed = Join-Path $Program 'hand.ps1'
[IO.File]::WriteAllBytes($installed, $handBytes)

# --- the queue (the member writes) and the answers (the member reads) ---
$queue = Join-Path $Root 'queue'
$done = Join-Path $Root 'done'
foreach ($folder in @($Root, $queue, $done)) { New-Item -ItemType Directory -Path $folder -Force | Out-Null }
$admins = '*S-1-5-32-544:(OI)(CI)F'
$system = '*S-1-5-18:(OI)(CI)F'
& icacls.exe $Root /inheritance:r /grant:r $admins $system ('*' + $userSid + ':(OI)(CI)RX') | Out-Null
& icacls.exe $queue /inheritance:r /grant:r $admins $system ('*' + $userSid + ':(OI)(CI)M') | Out-Null
& icacls.exe $done /inheritance:r /grant:r $admins $system ('*' + $userSid + ':(OI)(CI)RX') | Out-Null

# --- the three tasks ---
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 4)
$limited = New-ScheduledTaskPrincipal -UserId $User -LogonType Interactive -RunLevel Limited

$tick = New-ScheduledTaskAction -Execute $Castle -Argument '--sentinel-tick'
$every = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Minutes 5)
$logon = New-ScheduledTaskTrigger -AtLogOn -User $User
Register-ScheduledTask -TaskPath $TaskPath -TaskName 'Tick' -Action $tick -Trigger @($every, $logon) -Principal $limited -Settings $settings -Description 'MemoryCore Castle: the Sentinel looks at the antivirus and the firewall.' -Force | Out-Null

$query = '<QueryList><Query Id="0">' +
    '<Select Path="Microsoft-Windows-Windows Defender/Operational">*[System[(EventID=1116 or EventID=1117 or EventID=5001)]]</Select>' +
    '<Select Path="Microsoft-Windows-Windows Firewall With Advanced Security/Firewall">*[System[(EventID=2003)]]</Select>' +
    '</Query></QueryList>'
$eventClass = Get-CimClass -ClassName MSFT_TaskEventTrigger -Namespace 'Root/Microsoft/Windows/TaskScheduler'
$onEvent = New-CimInstance -CimClass $eventClass -ClientOnly
$onEvent.Enabled = $true
$onEvent.Subscription = $query
$alarm = New-ScheduledTaskAction -Execute $Castle -Argument '--sentinel-tick --cause alarm'
Register-ScheduledTask -TaskPath $TaskPath -TaskName 'Alarm' -Action $alarm -Trigger $onEvent -Principal $limited -Settings $settings -Description 'MemoryCore Castle: the Sentinel looks again when the antivirus or the firewall raises an alarm.' -Force | Out-Null

$handSettings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances Queue -ExecutionTimeLimit (New-TimeSpan -Minutes 10)
$elevated = New-ScheduledTaskPrincipal -UserId $User -LogonType S4U -RunLevel Highest
$handAction = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument ('-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $installed + '"')
Register-ScheduledTask -TaskPath $TaskPath -TaskName 'Hand' -Action $handAction -Principal $elevated -Settings $handSettings -Description 'MemoryCore Castle: the Sentinel''s hand. It only closes the computer further, and only by its lists.' -Force | Out-Null

# the member may start and read the three tasks (Castle starts the hand, and checks they exist)
try {
    $service = New-Object -ComObject 'Schedule.Service'
    $service.Connect()
    $folder = $service.GetFolder($TaskPath.TrimEnd('\'))
    foreach ($task in @('Tick', 'Alarm', 'Hand')) {
        $registered = $folder.GetTask($task)
        $sd = $registered.GetSecurityDescriptor(4)
        if ($sd -notlike ('*' + $userSid + '*')) { $registered.SetSecurityDescriptor($sd + '(A;;GRGX;;;' + $userSid + ')', 0) }
    }
} catch {
    Say 'The tasks were made, but the member could not be allowed to start them. Run the line again.'
    exit 3
}

$sha = [BitConverter]::ToString((New-Object Security.Cryptography.SHA256Managed).ComputeHash($handBytes)).Replace('-', '').ToLower()
Say "installed - the hand $sha, three tasks under $TaskPath, for $User."
exit 0
