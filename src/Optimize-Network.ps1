<#
.SYNOPSIS
    Optimizes network settings.
.DESCRIPTION
    Flushes DNS cache, resets Winsock, and resets TCP/IP stack.
#>
Write-Host "Starting Network Optimization..." -ForegroundColor Cyan

Write-Host "Flushing DNS Cache..." -ForegroundColor Yellow
ipconfig /flushdns

Write-Host "Resetting Winsock Catalog..." -ForegroundColor Yellow
netsh winsock reset

Write-Host "Resetting TCP/IP Stack..." -ForegroundColor Yellow
netsh int ip reset

Write-Host "`nNetwork Optimization Finished." -ForegroundColor Green
Write-Host "Note: A restart may be required for some network changes to take effect." -ForegroundColor Yellow
