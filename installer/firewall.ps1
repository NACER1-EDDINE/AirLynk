#Requires -RunAsAdministrator
<#
.SYNOPSIS
  Provisions the AirLynk inbound Windows Firewall rule before first launch (FR-28).
.DESCRIPTION
  Creates a program-scoped Allow rule so the user never meets the Windows Firewall
  prompt during normal use. Program scope means ephemeral-port fallback needs no
  second rule. Must run elevated — called by the signed installer, never by the app
  itself directly.
.PARAMETER ProgramPath
  Full path to airlynk.exe. Required.
.EXAMPLE
  .\firewall.ps1 -ProgramPath "C:\Program Files\AirLynk\airlynk.exe"
#>

param(
    [Parameter(Mandatory = $true)]
    [string]$ProgramPath
)

$ErrorActionPreference = "Stop"
$ruleName = "AirLynk (inbound LAN)"

# Idempotent: remove any prior rule with this name so a re-run or upgrade
# never creates duplicate rules.
Remove-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue

# Also remove any stale *block* rule that targets our program — Windows
# writes one when the user dismisses the prompt, and Block takes precedence
# over any Allow added later (FR-29).
$staleBlocks = Get-NetFirewallRule -Direction Inbound -Action Block -Enabled True -ErrorAction SilentlyContinue |
    Where-Object { $_.Program -and ($_.Program -eq $ProgramPath -or $_.Program -like "*airlynk*") }
foreach ($rule in $staleBlocks) {
    Write-Host "Removing stale block rule: $($rule.DisplayName)"
    Remove-NetFirewallRule -DisplayName $rule.DisplayName -ErrorAction SilentlyContinue
}

New-NetFirewallRule `
    -DisplayName $ruleName `
    -Description "Allows AirLynk to accept incoming file transfers from phones on the local network" `
    -Direction Inbound `
    -Program $ProgramPath `
    -Protocol TCP `
    -Action Allow `
    -Profile Private,Public `
    -Enabled True

Write-Host "Firewall rule '$ruleName' provisioned for $ProgramPath"