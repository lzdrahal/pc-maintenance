<#
.SYNOPSIS
    Repairs system files and Windows image.
.DESCRIPTION
    Runs SFC (System File Checker) and DISM (Deployment Image Servicing and Management) tools.
#>
Write-Host "Starting System Repair Checks..." -ForegroundColor Cyan
$hasError = $false

Write-Host "1. Running DISM /RestoreHealth (This may take a while)..." -ForegroundColor Yellow
& dism.exe /Online /Cleanup-Image /RestoreHealth
$dismExit = $LASTEXITCODE
if ($dismExit -eq 0) {
    Write-Host "DISM check completed." -ForegroundColor Green
} else {
    Write-Host "DISM exited with code $dismExit." -ForegroundColor Red
    $hasError = $true
}

Write-Host "`n2. Running SFC /scannow (System File Checker)..." -ForegroundColor Yellow
& sfc.exe /scannow
$sfcExit = $LASTEXITCODE
if ($sfcExit -eq 0) {
    Write-Host "SFC scan completed." -ForegroundColor Green
} else {
    Write-Host "SFC exited with code $sfcExit." -ForegroundColor Red
    $hasError = $true
}

if ($hasError) {
    Write-Host "`nSystem Repair finished with errors." -ForegroundColor Red
    exit 1
}

Write-Host "`nSystem Repair Operations Finished." -ForegroundColor Green
