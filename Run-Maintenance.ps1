<#
.SYNOPSIS
    Main entry point for the PC Maintenance Toolkit.
.DESCRIPTION
    Provides an interactive menu to run various maintenance tasks.
#>

# Ensure running as Administrator
$wid = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$prp = New-Object System.Security.Principal.WindowsPrincipal($wid)
$adm = [System.Security.Principal.WindowsBuiltInRole]::Administrator
if (-not $prp.IsInRole($adm)) {
    Write-Host "Elevating privileges... Please click 'Yes' on the UAC prompt." -ForegroundColor Yellow
    $newProcess = New-Object System.Diagnostics.ProcessStartInfo "PowerShell";
    $newProcess.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$($MyInvocation.MyCommand.Path)`""
    $newProcess.Verb = "runas";
    try {
        [System.Diagnostics.Process]::Start($newProcess)
    } catch {
        Write-Host "Failed to elevate privileges. The script must be run as Administrator." -ForegroundColor Red
        Pause
    }
    Exit
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

function Show-Menu {
    Clear-Host
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "       Windows PC Maintenance Toolkit    " -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "1. Update Windows"
    Write-Host "2. Update Software (via Winget)"
    Write-Host "3. Clean System Junk & Temp Files"
    Write-Host "4. Repair System Files (SFC & DISM)"
    Write-Host "5. Optimize Network Settings"
    Write-Host "-----------------------------------------"
    Write-Host "A. Run ALL Maintenance Tasks"
    Write-Host "Q. Quit"
    Write-Host "=========================================" -ForegroundColor Cyan
}

function Run-Task($TaskName, $ScriptFile) {
    Write-Host "`n--- Running Task: $TaskName ---" -ForegroundColor Magenta
    $ScriptPath = Join-Path $ScriptDir "src\$ScriptFile"
    if (Test-Path $ScriptPath) {
        & $ScriptPath
    } else {
        Write-Host "Error: Could not find $ScriptPath" -ForegroundColor Red
    }
    Write-Host "--- Task Completed ---`n" -ForegroundColor Magenta
}

while ($true) {
    Show-Menu
    $choice = Read-Host "Select an option"
    
    switch ($choice) {
        '1' { Run-Task "Update Windows" "Update-Windows.ps1"; Pause }
        '2' { Run-Task "Update Software" "Update-Software.ps1"; Pause }
        '3' { Run-Task "Clean System Junk" "Cleanup-System.ps1"; Pause }
        '4' { Run-Task "Repair System Files" "Repair-System.ps1"; Pause }
        '5' { Run-Task "Optimize Network Settings" "Optimize-Network.ps1"; Pause }
        'A' { 
            Run-Task "Update Windows" "Update-Windows.ps1"
            Run-Task "Update Software" "Update-Software.ps1"
            Run-Task "Clean System Junk" "Cleanup-System.ps1"
            Run-Task "Repair System Files" "Repair-System.ps1"
            Run-Task "Optimize Network Settings" "Optimize-Network.ps1"
            Write-Host "All tasks completed!" -ForegroundColor Green
            Pause
        }
        'Q' { Write-Host "Exiting..."; Exit }
        default { Write-Host "Invalid option. Please try again." -ForegroundColor Red; Start-Sleep -Seconds 2 }
    }
}
