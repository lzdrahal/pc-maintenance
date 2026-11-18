<#
.SYNOPSIS
    Upgrades installed software using Winget.
.DESCRIPTION
    This script utilizes the built-in Windows Package Manager (Winget) to find and install upgrades for all currently installed applications that support it.
#>
Write-Host "Checking for software updates via Winget..." -ForegroundColor Cyan

# Check if winget is available
if (Get-Command winget -ErrorAction SilentlyContinue) {
    Write-Host "Upgrading all available packages..." -ForegroundColor Yellow
    # --all: upgrade all packages
    # --include-unknown: include apps with unknown versions
    # --accept-source-agreements --accept-package-agreements: bypass prompts
    & winget upgrade --all --include-unknown --accept-source-agreements --accept-package-agreements
    $exitCode = $LASTEXITCODE

    # 0x8A15002B (-1978335189) = APPINSTALLER_CLI_ERROR_UPDATE_NOT_APPLICABLE (no updates available)
    if ($exitCode -eq 0) {
        Write-Host "Software update process completed." -ForegroundColor Green
    } elseif ($exitCode -eq -1978335189) {
        Write-Host "All Winget packages are up to date." -ForegroundColor Green
    } else {
        Write-Host "Winget exited with code $exitCode." -ForegroundColor Red
        exit $exitCode
    }
} else {
    Write-Host "Winget is not installed or not in the PATH. Skipping software updates." -ForegroundColor Red
    Write-Host "Please ensure you are running Windows 10 (1809 or newer) or Windows 11." -ForegroundColor Yellow
    exit 1
}
