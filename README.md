# Windows PC Maintenance Toolkit

A modern, graphical desktop application designed to automate and simplify routine maintenance tasks on a Windows PC. It provides a sleek user interface built on top of powerful PowerShell automation scripts.

## Download & Installation

The easiest way to use the toolkit is to download the compiled `.exe` file.

1. Go to the [Releases](../../releases) page of this repository.
2. Download the latest `PC Maintenance Toolkit Setup.exe` file.
3. Run the installer. The application will launch automatically and install a shortcut on your desktop.

## Features

### User Interface & Experience
- **Modern Dashboard**: A sleek, dark-mode user interface built with HTML/CSS and Electron, providing a much friendlier experience than raw terminal windows.
- **Real-Time Terminal Simulator**: Watch the live output of the PowerShell scripts as they run securely in the background.
- **Smart Auto-Updater**: The application silently checks GitHub for new releases when opened. If an update is found, it downloads in the background and prompts you to restart!

### Built-in Error Reporting
If a maintenance task encounters a critical failure, a **Report Issue** button will appear. Clicking this button automatically securely saves your session log and drafts an email with the exact error details so the developer can fix it.

### Core Maintenance Tasks
- **Windows Update**: Checks for, downloads, and installs available Windows updates.
- **Software Update**: Upgrades installed applications using the built-in `winget` package manager.
- **System Cleanup**: Empties temporary folders (User Temp, Windows Temp, Prefetch, SoftwareDistribution) and the Recycle Bin.
- **System Repair**: Runs standard system integrity checks (`sfc /scannow` and `DISM /RestoreHealth`).
- **Network Optimization**: Resets network caches, Winsock, and TCP/IP stack to resolve basic network issues.

---

## For Developers

The application architecture utilizes an Electron front-end running isolated PowerShell child-processes on the back-end.

### Local Development
To run the app in development mode on your machine:
```bash
# Install dependencies
npm install

# Start the application
npm start
```
*Note for Mac/Linux users: The app features a "Mock Mode." If you run `npm start` on macOS or Linux, the app will simulate fake PowerShell logs so you can develop the UI without crashing the application.*

### Automated Cloud Builds (CI/CD)
This repository is configured with **GitHub Actions**. You do not need to manually compile the `.exe` file on your computer.

To publish a new release to your users:
1. Make your code changes and commit them to the `main` or `master` branch.
2. Push your changes to GitHub.
3. The GitHub Actions server will automatically spin up a Windows machine, compile the `.exe`, and upload it to the Releases page.
4. Any users who have the app installed will automatically receive the update!

### Legacy Manual PowerShell Usage
If you prefer not to use the GUI, the core PowerShell scripts are still entirely intact and usable as standalone files.
1. Open an Administrator PowerShell window.
2. Navigate to the `/src` folder.
3. Run `.\Run-Maintenance.ps1` to launch the interactive terminal menu.
