<#
.SYNOPSIS
    Cleans up temporary files and system junk.
.DESCRIPTION
    Empties user temporary folders, Windows temporary folders, and the Recycle Bin.
#>
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

Write-Host "Emptying Recycle Bin..." -ForegroundColor Yellow
try {
    Clear-RecycleBin -Force -ErrorAction SilentlyContinue
    Write-Host " - Done." -ForegroundColor Green
} catch {
    Write-Host " - Failed to empty Recycle Bin." -ForegroundColor Red
}

Write-Host "System Cleanup Finished." -ForegroundColor Green
