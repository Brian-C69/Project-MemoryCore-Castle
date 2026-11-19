# The Sentinel's look: a read-only look at this computer, printed as one JSON document.
# It changes nothing and never opens a file's contents. -Quick stops after the antivirus
# protection, the firewall and recent detections (the watch's five-minute look).
# Windows PowerShell 5.1. ASCII only.
param([switch]$Quick)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Text($value) {
    if ($null -eq $value) { return '' }
    return ([string]$value).Trim()
}

$out = [ordered]@{ quick = [bool]$Quick; problems = @() }

# --- the antivirus products Windows knows about ---
$products = @()
try {
    foreach ($p in Get-CimInstance -Namespace 'root/SecurityCenter2' -ClassName AntiVirusProduct) {
        $products += [ordered]@{ name = (Text $p.displayName); state = [int]$p.productState }
    }
} catch { $out.problems += 'antivirus products' }
$out.products = $products

# --- Microsoft Defender ---
try {
    $s = Get-MpComputerStatus
    $age = -1
    if ($null -ne $s.AntivirusSignatureAge) { $age = [int]$s.AntivirusSignatureAge }
    $scan = ''
    if ($s.QuickScanEndTime) { $scan = $s.QuickScanEndTime.ToString('yyyy-MM-dd HH:mm') }
    $out.defender = [ordered]@{
        available = $true
        mode = (Text $s.AMRunningMode)
        realtime = [bool]$s.RealTimeProtectionEnabled
        antivirus = [bool]$s.AntivirusEnabled
        tamper = [bool]$s.IsTamperProtected
        signatureAgeDays = $age
        quickScanEnd = $scan
    }
} catch {
    $out.defender = [ordered]@{ available = $false }
}

# --- detections in the last seven days ---
$detections = @()
if ($out.defender.available) {
    try {
        $names = @{}
        foreach ($t in Get-MpThreat) { $names[[string]$t.ThreatID] = (Text $t.ThreatName) }
        $since = (Get-Date).AddDays(-7)
        foreach ($d in Get-MpThreatDetection) {
            if ($d.InitialDetectionTime -lt $since) { continue }
            $resources = @()
            foreach ($r in $d.Resources) { $resources += (Text $r) }
            $detections += [ordered]@{
                id = (Text $d.DetectionID)
                threat = $names[[string]$d.ThreatID]
                resources = ($resources -join ' | ')
                handled = [bool]$d.ActionSuccess
                at = $d.InitialDetectionTime.ToString('yyyy-MM-dd HH:mm')
            }
        }
    } catch { $out.problems += 'detections' }
}
$out.detections = $detections

# --- the firewall's profiles ---
$firewall = @()
try {
    foreach ($f in Get-NetFirewallProfile) {
        $firewall += [ordered]@{ name = (Text $f.Name); enabled = ([string]$f.Enabled -eq 'True') }
    }
} catch { $out.problems += 'firewall' }
$out.firewall = $firewall

if ($Quick) {
    $out | ConvertTo-Json -Depth 5 -Compress
    exit 0
}

# --- what starts with the system ---
$startup = @()
try {
    foreach ($e in Get-CimInstance -ClassName Win32_StartupCommand) {
        $startup += [ordered]@{ name = (Text $e.Name); command = (Text $e.Command); location = (Text $e.Location); user = (Text $e.User) }
    }
} catch { $out.problems += 'startup' }
$out.startup = $startup

# --- scheduled tasks ---
$tasks = @()
try {
    foreach ($t in Get-ScheduledTask) {
        $actions = @()
        foreach ($a in $t.Actions) {
            if ($a.Execute) { $actions += ((Text $a.Execute) + ' ' + (Text $a.Arguments)).Trim() }
            elseif ($a.ClassId) { $actions += (Text $a.ClassId) }
        }
        $tasks += [ordered]@{
            path = (Text $t.TaskPath)
            name = (Text $t.TaskName)
            state = (Text $t.State)
            author = (Text $t.Author)
            actions = ($actions -join ' | ')
        }
    }
} catch { $out.problems += 'tasks' }
$out.tasks = $tasks

# --- services installed in the last seven days (System log, event 7045) ---
$services = @()
try {
    $events = Get-WinEvent -FilterHashtable @{ LogName = 'System'; Id = 7045; StartTime = (Get-Date).AddDays(-7) } -ErrorAction Stop
    foreach ($e in $events) {
        $p = $e.Properties
        $services += [ordered]@{
            name = (Text $p[0].Value)
            path = (Text $p[1].Value)
            account = (Text $p[4].Value)
            at = $e.TimeCreated.ToString('yyyy-MM-dd HH:mm')
        }
    }
} catch {
    # no event in the window is not a problem
    if ($_.FullyQualifiedErrorId -notlike 'NoMatchingEventsFound*') { $out.problems += 'services' }
}
$out.services = $services

# --- local accounts and administrators (by SID, so any Windows language works) ---
$users = @()
try {
    foreach ($u in Get-LocalUser) { $users += [ordered]@{ name = (Text $u.Name); enabled = [bool]$u.Enabled } }
} catch { $out.problems += 'accounts' }
$out.users = $users

$admins = @()
try {
    foreach ($m in Get-LocalGroupMember -SID 'S-1-5-32-544') { $admins += [ordered]@{ name = (Text $m.Name) } }
} catch {
    try {
        $group = (New-Object System.Security.Principal.SecurityIdentifier('S-1-5-32-544')).Translate([System.Security.Principal.NTAccount]).Value.Split('\')[-1]
        foreach ($m in ([ADSI]("WinNT://./" + $group + ",group")).Invoke('Members')) {
            $admins += [ordered]@{ name = (Text $m.GetType().InvokeMember('Name', 'GetProperty', $null, $m, $null)) }
        }
    } catch { $out.problems += 'administrators' }
}
$out.admins = $admins

# --- remote desktop ---
try {
    $deny = (Get-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name fDenyTSConnections).fDenyTSConnections
    $out.rdp = [ordered]@{ on = ([int]$deny -eq 0) }
} catch {
    $out.problems += 'remote desktop'
    $out.rdp = [ordered]@{ on = $false }
}

$out | ConvertTo-Json -Depth 5 -Compress
