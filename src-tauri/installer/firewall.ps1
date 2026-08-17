$ErrorActionPreference = 'Stop'

$program = $env:AIRLYNK_PROGRAM_PATH
if (-not $program) {
    $program = (Get-Process -Id $PID -ErrorAction SilentlyContinue).Path
}
if (-not $program) {
    $program = (Get-Command airlynk -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
}
if (-not $program) {
    throw 'AirLynk executable path is required.'
}

$allowRule = 'AirLynk (inbound LAN)'
$blockRules = @(Get-NetFirewallRule -Direction Inbound -Action Block -ErrorAction SilentlyContinue | Where-Object { $_.Program -and $_.Program -like '*airlynk*' })
foreach ($rule in $blockRules) {
    try {
        Remove-NetFirewallRule -DisplayName $rule.DisplayName -ErrorAction SilentlyContinue
    } catch {}
}

try {
    Remove-NetFirewallRule -DisplayName $allowRule -ErrorAction SilentlyContinue
} catch {}

New-NetFirewallRule -DisplayName $allowRule `
    -Description 'Allows AirLynk to accept incoming file transfers from phones on the local network' `
    -Direction Inbound -Program $program -Protocol TCP -Action Allow `
    -Profile Private,Public -Enabled True | Out-Null

Write-Output "AirLynk firewall rule ready"
