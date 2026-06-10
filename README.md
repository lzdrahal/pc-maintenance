# Windows PC Maintenance Toolkit

A collection of PowerShell scripts to automate and simplify routine maintenance tasks on a Windows PC.

## Features

- **Windows Update**: Checks for, downloads, and installs available Windows updates.
- **Software Update**: Upgrades installed applications using the built-in `winget` package manager.
- **System Cleanup**: Empties temporary folders (User Temp, Windows Temp, Prefetch, SoftwareDistribution) and the Recycle Bin.
- **System Repair**: Runs standard system integrity checks (`sfc /scannow` and `DISM /RestoreHealth`).
- **Network Optimization**: Resets network caches, Winsock, and TCP/IP stack to resolve basic network issues.

## Usage

**Easiest Method (One-Line Command)**
You can run this single command in an Administrator PowerShell window to automatically download, extract, and launch the toolkit without any manual steps:
```powershell
Invoke-WebRequest -Uri "https://github.com/lzdrahal/pc-maintenance/archive/refs/heads/master.zip" -OutFile "$env:TEMP\maintenance.zip"; Expand-Archive "$env:TEMP\maintenance.zip" -DestinationPath "$env:TEMP\maintenance" -Force; Set-Location "$env:TEMP\maintenance\pc-maintenance-master"; .\Run-Maintenance.ps1
```

**Manual Method**
1. Copy or clone this repository to your target Windows PC.
2. Open the folder containing the repository.
3. Right-click on `Run-Maintenance.ps1` and select **Run with PowerShell**.
   - *Note: You may be prompted by User Account Control (UAC) to grant Administrator privileges, as these tasks require elevated permissions to run effectively.*
4. Follow the interactive menu to select which maintenance tasks to perform, or choose to run them all sequentially.

## Troubleshooting

- **Execution Policy**: If the script closes immediately or throws an error about execution policies, you might need to allow script execution. Open an Administrator PowerShell window and run:
  `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser`
- **Winget Not Found**: The software update module requires Windows Package Manager (`winget`), which is built into modern versions of Windows 10 and 11. If it's missing, you may need to update your App Installer via the Microsoft Store.
