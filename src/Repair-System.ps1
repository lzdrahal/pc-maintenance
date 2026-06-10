<#
.SYNOPSIS
    Repairs system files and Windows image.
.DESCRIPTION
    Runs SFC (System File Checker) and DISM (Deployment Image Servicing and Management) tools.
#>
Write-Host "Starting System Repair Checks..." -ForegroundColor Cyan

Write-Host "1. Running DISM /RestoreHealth (This may take a while)..." -ForegroundColor Yellow
try {
    # /Online targets the running operating system
    # /Cleanup-Image performs cleanup operations on the image
    # /RestoreHealth scans the image for component store corruption and performs repair operations
    Start-Process -FilePath "dism.exe" -ArgumentList "/Online /Cleanup-Image /RestoreHealth" -Wait -NoNewWindow
    Write-Host "DISM check completed." -ForegroundColor Green
} catch {
    Write-Host "Error running DISM." -ForegroundColor Red
}

Write-Host "`n2. Running SFC /scannow (System File Checker)..." -ForegroundColor Yellow
try {
    Start-Process -FilePath "sfc.exe" -ArgumentList "/scannow" -Wait -NoNewWindow
    Write-Host "SFC scan completed." -ForegroundColor Green
} catch {
    Write-Host "Error running SFC." -ForegroundColor Red
}

Write-Host "`nSystem Repair Operations Finished." -ForegroundColor Green
