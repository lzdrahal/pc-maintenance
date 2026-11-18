# System Maintenance Toolkit

A user-friendly, cross-platform desktop application for routine **macOS** and **Windows** maintenance, built with **Tauri v2**.

## Features

- **OS-Aware Guided Dashboard**: Automatically detects macOS (Apple Silicon / Intel) or Windows, package managers, free disk space, and privileges.
- **Selective Execution & Live Controls**: Pick individual tasks with checkboxes, configure safe inline options, run one or many tasks sequentially, or cancel active work mid-run.
- **Clear Summaries + Collapsible Technical Output**: Plain-English completion summaries (including space freed and restart hints) alongside a collapsible live log viewer with copy and clear actions.
- **System-Matched Light & Dark Mode**: Automatically follows your OS theme with a manual header toggle.
- **Non-Blocking Update Check**: Checks GitHub Releases on launch and links directly to new versions when available.
- **One-Click Issue Reporting**: Saves a diagnostic session log (`maintenance-error.log`), opens its folder, and prepares an email report when a task fails.

### macOS Tasks
- **Update macOS** (`Admin`): Checks `softwareupdate --list` and installs recommended updates via native macOS authorization prompt (includes a shortcut to System Settings if Apple Silicon interactive confirmation is required).
- **Update Homebrew Packages** *(auto-detected — shown only when `brew` is installed)*: Runs `brew update`, `brew upgrade --formula`, optional `brew upgrade --cask` (default ON, without `--greedy`), `brew autoremove`, and `brew cleanup` as your normal user.
- **Clean System Junk**: Safely clears non-Apple caches and logs in `~/Library/Caches` and `~/Library/Logs` while skipping protected system directories. Optional **Empty Trash** toggle (off by default, requires confirmation).
- **Disk & System Health**: Verifies startup volume integrity (`diskutil verifyVolume /`), checks drive SMART status, and runs `brew doctor` when Homebrew is installed (Homebrew warnings are reported as non-fatal warnings).
- **Optimize Network** (`Admin`): Flushes macOS DNS cache and refreshes `mDNSResponder`.

### Windows Tasks
- **Update Windows** (`Admin`): Searches for, downloads, and installs available Windows updates.
- **Update Software (Winget)**: Upgrades installed applications via `winget` (automatically treats "no updates available" as up to date).
- **Clean System Junk** (`Admin`): Cleans user/Windows temp folders, Prefetch, and Windows Update download cache. Optional **Empty Recycle Bin** toggle (off by default, requires confirmation).
- **Repair System Files** (`Admin`): Runs `DISM /RestoreHealth` and `sfc /scannow` with live streaming output.
- **Optimize Network** (`Admin`): Flushes DNS cache and resets Winsock and TCP/IP stack.
- Includes a **Restart as Admin** button when launched without Administrator privileges.

---

## Download & Installation

1. Open the [Releases](../../releases) page.
2. Download the installer for your OS:
   - **macOS (Apple Silicon & Intel)**: `System Maintenance Toolkit_x.y.z_universal.dmg` (or `.app.zip`)
   - **Windows (x64)**: `System Maintenance Toolkit_x.y.z_x64-setup.exe`

### First Launch on Unsigned macOS Builds
Because the macOS build is unsigned (no Apple Developer ID certificate), Gatekeeper may block double-clicking on first launch:
1. Drag **System Maintenance Toolkit.app** into **Applications**.
2. **Right-click** (or Control-click) the app and choose **Open**, then confirm **Open**.
   - On newer macOS versions if blocked immediately: open **System Settings → Privacy & Security**, scroll down, and click **Open Anyway**.

---

## Development

### Prerequisites
- **Node.js 22+**
- **Rust 1.90+** (`rustup update stable`)
- Platform prerequisites for Tauri v2 (macOS: Xcode Command Line Tools; Windows: WebView2 + Visual Studio C++ Build Tools)

### Commands
```bash
# Install Tauri CLI
npm install

# Run in development mode
npm run dev

# Run Rust backend unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Build distributable installers for the current OS
npm run build
```

### Publishing a Release (GitHub Actions)
1. Update the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Commit and push to `master` (CI verifies tests and builds on macOS + Windows).
3. Tag and push to publish the GitHub Release with Universal macOS DMG/ZIP and Windows NSIS installer:
   ```bash
   git tag v2.0.0
   git push --tags
   ```

### Legacy Windows PowerShell CLI
The standalone Windows PowerShell scripts remain in `src/` and can still be run directly in an elevated PowerShell window via `.\Run-Maintenance.ps1`.
