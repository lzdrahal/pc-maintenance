<#
.SYNOPSIS
    Cleans up temporary files and system junk.
.DESCRIPTION
    Empties user temporary folders, Windows temporary folders, and optionally the Recycle Bin.
#>
param(
    [switch]$SkipRecycleBin
)

Write-Host "Starting System Cleanup..." -ForegroundColor Cyan

$TempFolders = @(
    "$env:TEMP",
    "$env:WINDIR\Temp",
    "$env:WINDIR\Prefetch",
    "$env:WINDIR\SoftwareDistribution\Download"
)

foreach ($Folder in $TempFolders) {
    if (Test-Path $Folder) {
        Write-Host "Cleaning $Folder..." -ForegroundColor Yellow
        try {
            Get-ChildItem -Path $Folder -Recurse -Force -ErrorAction SilentlyContinue |
                Where-Object { -not $_.PSIsContainer } |
                Remove-Item -Force -ErrorAction SilentlyContinue
            Write-Host " - Done." -ForegroundColor Green
        } catch {
            Write-Host " - Failed to clean some files in $Folder. (They might be in use)" -ForegroundColor Red
        }
    }
}

if (-not $SkipRecycleBin) {
    Write-Host "Emptying Recycle Bin..." -ForegroundColor Yellow
    try {
        Clear-RecycleBin -Force -ErrorAction SilentlyContinue
        Write-Host " - Done." -ForegroundColor Green
    } catch {
        Write-Host " - Failed to empty Recycle Bin." -ForegroundColor Red
    }
} else {
    Write-Host "Skipping Recycle Bin cleanup." -ForegroundColor Yellow
}

Write-Host "System Cleanup Finished." -ForegroundColor Green
