<#
.SYNOPSIS
    Installs available Windows Updates.
.DESCRIPTION
    Uses the built-in Microsoft.Update.Session COM object to search for, download, and install available Windows updates.
#>
Write-Host "Checking for Windows Updates..." -ForegroundColor Cyan

$UpdateSession = New-Object -ComObject Microsoft.Update.Session
$UpdateSearcher = $UpdateSession.CreateUpdateSearcher()

Write-Host "Searching for available updates. This may take a few minutes..."
$SearchResult = $UpdateSearcher.Search("IsInstalled=0 and Type='Software' and IsHidden=0")

if ($SearchResult.Updates.Count -eq 0) {
    Write-Host "No new Windows updates found. Your system is up to date." -ForegroundColor Green
    return
}

Write-Host "Found $($SearchResult.Updates.Count) update(s):" -ForegroundColor Yellow
$UpdatesToDownload = New-Object -ComObject Microsoft.Update.UpdateColl

foreach ($Update in $SearchResult.Updates) {
    Write-Host " - $($Update.Title)"
    $UpdatesToDownload.Add($Update) | Out-Null
}

Write-Host "`nDownloading updates..." -ForegroundColor Cyan
$Downloader = $UpdateSession.CreateUpdateDownloader()
$Downloader.Updates = $UpdatesToDownload
$DownloadResult = $Downloader.Download()

Write-Host "`nInstalling updates..." -ForegroundColor Cyan
$UpdatesToInstall = New-Object -ComObject Microsoft.Update.UpdateColl

foreach ($Update in $SearchResult.Updates) {
    if ($Update.IsDownloaded) {
        $UpdatesToInstall.Add($Update) | Out-Null
    }
}

if ($UpdatesToInstall.Count -gt 0) {
    $Installer = $UpdateSession.CreateUpdateInstaller()
    $Installer.Updates = $UpdatesToInstall
    $InstallResult = $Installer.Install()
    
    Write-Host "`nInstallation finished." -ForegroundColor Green
    if ($InstallResult.RebootRequired) {
        Write-Host "A system reboot is required to complete the installation." -ForegroundColor Red
    }
} else {
    Write-Host "Failed to download updates." -ForegroundColor Red
}
