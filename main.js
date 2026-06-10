const { app, BrowserWindow, ipcMain, shell } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const { autoUpdater } = require('electron-updater');

let mainWindow;
let currentSessionLog = '';

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 1000,
        height: 700,
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'),
            nodeIntegration: false,
            contextIsolation: true
        },
        autoHideMenuBar: true,
        backgroundColor: '#0f172a'
    });

    mainWindow.loadFile('index.html');
}

function setupUpdater(window) {
    autoUpdater.on('checking-for-update', () => {
        window.webContents.send('update-message', 'Checking for updates...');
    });
    autoUpdater.on('update-available', (info) => {
        window.webContents.send('update-message', `Update v${info.version} available! Downloading...`);
    });
    autoUpdater.on('update-not-available', (info) => {
        window.webContents.send('update-message', 'App is up to date.');
    });
    autoUpdater.on('error', (err) => {
        window.webContents.send('update-message', `Error in auto-updater: ${err.message}`);
    });
    autoUpdater.on('update-downloaded', (info) => {
        window.webContents.send('update-message', `Update downloaded. Restarting to install...`);
        setTimeout(() => {
            autoUpdater.quitAndInstall();
        }, 3000);
    });
}

app.whenReady().then(() => {
    createWindow();
    
    // Only check for updates in production (packaged) apps
    if (app.isPackaged) {
        setupUpdater(mainWindow);
        autoUpdater.checkForUpdatesAndNotify();
    } else {
        // Mock update message in dev
        setTimeout(() => mainWindow.webContents.send('update-message', 'Running in dev mode. Auto-updater disabled.'), 2000);
    }

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) {
            createWindow();
        }
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

// Handle IPC
ipcMain.handle('report-error', async () => {
    const logPath = path.join(app.getPath('userData'), 'error.log');
    try {
        fs.writeFileSync(logPath, `=== PC Maintenance Error Log ===\nDate: ${new Date().toISOString()}\nOS: ${os.type()} ${os.release()}\n\n${currentSessionLog}`);
        shell.showItemInFolder(logPath);
        shell.openExternal('mailto:lzdrahal@gmail.com?subject=PC Maintenance Toolkit Error Report&body=An error occurred. I have attached the error.log file that was just opened in my file explorer.');
    } catch (e) {
        console.error("Failed to write log file", e);
    }
});

ipcMain.handle('run-task', async (event, taskName, scriptFile) => {
    currentSessionLog += `\n\n--- Starting Task: ${taskName} ---\n`;
    if (process.platform === 'darwin' || process.platform === 'linux') {
        // Mac/Linux Mock Mode
        return new Promise((resolve) => {
            let i = 0;
            const interval = setInterval(() => {
                i++;
                const text = `[Mock Output] Running ${taskName} - Step ${i}...\n`;
                currentSessionLog += text;
                event.sender.send('task-output', text);

                // Simulate an error on task 3 for testing the report button
                if (i >= 3 && taskName.includes("Clean System Junk")) {
                    clearInterval(interval);
                    const errText = `[Mock Error] Failed to complete ${taskName}!\n`;
                    currentSessionLog += errText;
                    event.sender.send('task-output', errText);
                    event.sender.send('task-complete', false);
                    resolve(false);
                    return;
                }

                if (i >= 5) {
                    clearInterval(interval);
                    const finishText = `Task ${taskName} completed successfully.\n`;
                    currentSessionLog += finishText;
                    event.sender.send('task-output', finishText);
                    event.sender.send('task-complete', true);
                    resolve(true);
                }
            }, 500);
        });
    }

    // Windows Real Mode
    return new Promise((resolve) => {
        const scriptPath = path.join(__dirname, 'src', scriptFile);

        event.sender.send('task-output', `Starting ${taskName}...\n`);

        const ps = spawn('powershell.exe', [
            '-NoProfile',
            '-ExecutionPolicy', 'Bypass',
            '-File', scriptPath
        ]);

        ps.stdout.on('data', (data) => {
            const text = data.toString();
            currentSessionLog += text;
            event.sender.send('task-output', text);
        });

        ps.stderr.on('data', (data) => {
            const text = `ERROR: ${data.toString()}`;
            currentSessionLog += text;
            event.sender.send('task-output', text);
        });

        ps.on('close', (code) => {
            const text = `\nTask exited with code ${code}\n`;
            currentSessionLog += text;
            event.sender.send('task-output', text);
            event.sender.send('task-complete', code === 0);
            resolve(code === 0);
        });

        ps.on('error', (err) => {
            const text = `Failed to start: ${err.message}\n`;
            currentSessionLog += text;
            event.sender.send('task-output', text);
            event.sender.send('task-complete', false);
            resolve(false);
        });
    });
});
