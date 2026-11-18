(() => {
  const STORAGE_KEY = "smt-color-scheme";
  const MAX_DOM_LOG_LINES = 4000;

  const tauri = window.__TAURI__;
  const invoke = tauri?.core?.invoke;
  const listen = tauri?.event?.listen;

  const state = {
    systemInfo: null,
    tasks: [],
    selectedIds: new Set(),
    optionValues: new Map(),
    taskStates: new Map(),
    isRunning: false,
    cancelRequested: false,
    logLineCount: 0,
    updateInfo: null,
  };

  const els = {
    appVersion: document.getElementById("app-version"),
    themeToggle: document.getElementById("theme-toggle"),
    updateBanner: document.getElementById("update-banner"),
    updateTitle: document.getElementById("update-title"),
    updateMessage: document.getElementById("update-message"),
    updateDownloadBtn: document.getElementById("update-download-btn"),
    updateDismissBtn: document.getElementById("update-dismiss-btn"),
    sysOs: document.getElementById("sys-os"),
    sysPkg: document.getElementById("sys-pkg"),
    sysDisk: document.getElementById("sys-disk"),
    sysAdmin: document.getElementById("sys-admin"),
    relaunchAdminBtn: document.getElementById("relaunch-admin-btn"),
    selectAllBtn: document.getElementById("select-all-btn"),
    selectNoneBtn: document.getElementById("select-none-btn"),
    selectionCount: document.getElementById("selection-count"),
    runSelectedBtn: document.getElementById("run-selected-btn"),
    cancelBtn: document.getElementById("cancel-btn"),
    summaryBanner: document.getElementById("summary-banner"),
    summaryTitle: document.getElementById("summary-title"),
    summaryText: document.getElementById("summary-text"),
    summarySettingsBtn: document.getElementById("summary-settings-btn"),
    reportIssueBtn: document.getElementById("report-issue-btn"),
    taskList: document.getElementById("task-list"),
    logPanel: document.getElementById("log-panel"),
    logCount: document.getElementById("log-count"),
    autoscrollToggle: document.getElementById("autoscroll-toggle"),
    copyLogBtn: document.getElementById("copy-log-btn"),
    clearLogBtn: document.getElementById("clear-log-btn"),
    terminalOutput: document.getElementById("terminal-output"),
    confirmDialog: document.getElementById("confirm-dialog"),
    confirmMessage: document.getElementById("confirm-message"),
  };

  function formatBytes(bytes) {
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
    const gb = 1024 * 1024 * 1024;
    const mb = 1024 * 1024;
    const kb = 1024;
    if (bytes >= gb) return `${(bytes / gb).toFixed(1)} GB`;
    if (bytes >= mb) return `${(bytes / mb).toFixed(1)} MB`;
    if (bytes >= kb) return `${Math.round(bytes / kb)} KB`;
    return `${bytes} B`;
  }

  /* Color scheme two-state toggle */
  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const colorSchemeMeta = document.querySelector('meta[name="color-scheme"]');

  function syncThemeMetaFromStorage() {
    let stored = null;
    try {
      stored = localStorage.getItem(STORAGE_KEY);
    } catch (_err) {
      stored = null;
    }
    if (colorSchemeMeta) {
      colorSchemeMeta.content = stored === "light" || stored === "dark" ? stored : "light dark";
    }
  }

  function isCurrentlyRenderedDark() {
    const content = colorSchemeMeta?.content || "light dark";
    if (content === "dark") return true;
    if (content === "light") return false;
    return darkQuery.matches;
  }

  els.themeToggle.addEventListener("click", () => {
    const target = isCurrentlyRenderedDark() ? "light" : "dark";
    const systemPref = darkQuery.matches ? "dark" : "light";
    try {
      if (target === systemPref) {
        localStorage.removeItem(STORAGE_KEY);
      } else {
        localStorage.setItem(STORAGE_KEY, target);
      }
    } catch (_err) {
      // Fall back to runtime meta update even if storage is unavailable
    }
    if (colorSchemeMeta) {
      colorSchemeMeta.content = target === systemPref ? "light dark" : target;
    }
  });

  window.addEventListener("storage", (e) => {
    if (e.key === STORAGE_KEY) {
      syncThemeMetaFromStorage();
    }
  });

  /* Logging */
  function appendLogLine(text, stream = "stdout") {
    if (!text) return;
    if (state.logLineCount === 0 && els.terminalOutput.firstElementChild?.textContent?.startsWith("Ready.")) {
      els.terminalOutput.innerHTML = "";
    }

    const line = document.createElement("div");
    line.className = `log-line log-${stream}`;
    line.textContent = text;
    els.terminalOutput.appendChild(line);

    state.logLineCount += 1;
    while (els.terminalOutput.childElementCount > MAX_DOM_LOG_LINES) {
      els.terminalOutput.removeChild(els.terminalOutput.firstElementChild);
    }

    els.logCount.textContent = `${state.logLineCount} ${state.logLineCount === 1 ? "line" : "lines"}`;
    if (els.autoscrollToggle.checked) {
      els.terminalOutput.scrollTop = els.terminalOutput.scrollHeight;
    }
  }

  els.copyLogBtn.addEventListener("click", async () => {
    try {
      const fullLog = invoke ? await invoke("get_session_log") : els.terminalOutput.innerText;
      await navigator.clipboard.writeText(fullLog || els.terminalOutput.innerText);
      const prev = els.copyLogBtn.textContent;
      els.copyLogBtn.textContent = "Copied";
      setTimeout(() => {
        els.copyLogBtn.textContent = prev;
      }, 1500);
    } catch (err) {
      appendLogLine(`Failed to copy log: ${err}`, "stderr");
    }
  });

  els.clearLogBtn.addEventListener("click", async () => {
    els.terminalOutput.innerHTML = '<div class="log-line log-info">Log cleared.</div>';
    state.logLineCount = 0;
    els.logCount.textContent = "0 lines";
    if (invoke) {
      try {
        await invoke("clear_session_log");
      } catch (_err) {
        // ignore
      }
    }
  });

  /* System info & Update banner */
  function renderSystemInfo(info) {
    state.systemInfo = info;
    els.appVersion.textContent = `v${info.appVersion}`;
    els.sysOs.textContent = `${info.osVersion} · ${info.arch}`;

    if (info.os === "macos") {
      if (info.brewPath) {
        els.sysPkg.textContent = info.brewVersion ? `Homebrew ${info.brewVersion}` : "Homebrew installed";
        els.sysPkg.className = "stat-value is-good";
      } else {
        els.sysPkg.textContent = "Homebrew not installed";
        els.sysPkg.className = "stat-value is-muted";
      }
    } else if (info.os === "windows") {
      if (info.wingetAvailable) {
        els.sysPkg.textContent = info.wingetVersion ? `Winget ${info.wingetVersion}` : "Winget available";
        els.sysPkg.className = "stat-value is-good";
      } else {
        els.sysPkg.textContent = "Winget unavailable";
        els.sysPkg.className = "stat-value is-warn";
      }
    } else {
      els.sysPkg.textContent = "Unsupported OS";
      els.sysPkg.className = "stat-value is-muted";
    }

    if (info.totalDiskBytes > 0) {
      els.sysDisk.textContent = `${formatBytes(info.freeDiskBytes)} free of ${formatBytes(info.totalDiskBytes)}`;
    } else {
      els.sysDisk.textContent = "Unavailable";
    }

    if (info.os === "windows") {
      if (info.isAdmin) {
        els.sysAdmin.textContent = "Administrator";
        els.sysAdmin.className = "stat-value is-good";
        els.relaunchAdminBtn.classList.add("hidden");
      } else {
        els.sysAdmin.textContent = "Standard User";
        els.sysAdmin.className = "stat-value is-warn";
        els.relaunchAdminBtn.classList.remove("hidden");
      }
    } else {
      els.relaunchAdminBtn.classList.add("hidden");
      els.sysAdmin.textContent = info.isAdmin ? "Root" : "Standard (prompts when needed)";
      els.sysAdmin.className = "stat-value";
    }
  }

  async function refreshSystemInfo() {
    if (!invoke) return;
    try {
      const info = await invoke("get_system_info");
      renderSystemInfo(info);
    } catch (err) {
      appendLogLine(`Failed to refresh system info: ${err}`, "stderr");
    }
  }

  async function checkUpdatesNonBlocking() {
    if (!invoke) return;
    try {
      const update = await invoke("check_updates");
      if (!update) return;
      state.updateInfo = update;
      els.updateTitle.textContent = `Update v${update.latestVersion} available`;
      els.updateMessage.textContent = `You are running v${update.currentVersion}. Download the latest release from GitHub.`;
      els.updateBanner.classList.remove("hidden");
    } catch (_err) {
      // Update check is strictly non-blocking and silent on error.
    }
  }

  els.updateDownloadBtn.addEventListener("click", async () => {
    if (!invoke || !state.updateInfo?.releaseUrl) return;
    try {
      await invoke("open_external_url", { url: state.updateInfo.releaseUrl });
    } catch (err) {
      appendLogLine(`Could not open release page: ${err}`, "stderr");
    }
  });

  els.updateDismissBtn.addEventListener("click", () => {
    els.updateBanner.classList.add("hidden");
  });

  els.relaunchAdminBtn.addEventListener("click", async () => {
    if (!invoke) return;
    try {
      await invoke("relaunch_as_admin");
    } catch (err) {
      appendLogLine(`Could not relaunch as Administrator: ${err}`, "stderr");
    }
  });

  /* Confirmation <dialog> */
  function confirmDestructiveAction(labels) {
    return new Promise((resolve) => {
      const list = labels.join(", ");
      els.confirmMessage.textContent = `The following permanent cleanup option is enabled: ${list}. Deleted items cannot be recovered.`;

      const handleClose = () => {
        const result = els.confirmDialog.returnValue;
        els.confirmDialog.returnValue = "";
        els.confirmDialog.removeEventListener("close", handleClose);
        resolve(result === "confirm");
      };

      els.confirmDialog.addEventListener("close", handleClose);
      els.confirmDialog.showModal();
    });
  }

  /* Task Rendering */
  function updateToolbar() {
    const selectable = state.tasks.filter((t) => t.available);
    const count = selectable.filter((t) => state.selectedIds.has(t.id)).length;
    els.selectionCount.textContent = `${count} selected`;
    els.runSelectedBtn.textContent = state.isRunning ? "Running..." : `Run Selected (${count})`;
    els.runSelectedBtn.disabled = state.isRunning || count === 0;
    els.selectAllBtn.disabled = state.isRunning || selectable.length === 0;
    els.selectNoneBtn.disabled = state.isRunning || count === 0;
    els.cancelBtn.classList.toggle("hidden", !state.isRunning);
    els.cancelBtn.disabled = !state.isRunning || state.cancelRequested;
    els.relaunchAdminBtn.disabled = state.isRunning;
  }

  function statusLabel(status) {
    switch (status) {
      case "queued": return "Queued";
      case "running": return "Running";
      case "success": return "Done";
      case "warning": return "Warning";
      case "failed": return "Failed";
      case "canceled": return "Canceled";
      default: return "Ready";
    }
  }

  function renderTasks() {
    if (!state.tasks.length) {
      els.taskList.innerHTML = '<div class="loading-state">No maintenance tasks available for this platform.</div>';
      updateToolbar();
      return;
    }

    const fragment = document.createDocumentFragment();

    for (const task of state.tasks) {
      const runtime = state.taskStates.get(task.id) || { status: "idle", result: null };
      const card = document.createElement("article");
      card.className = "task-card";
      if (runtime.status === "running") card.classList.add("is-running");
      if (!task.available) card.classList.add("is-unavailable");

      // Checkbox column
      const checkWrap = document.createElement("div");
      checkWrap.className = "task-checkbox-wrap";
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.id = `task-check-${task.id}`;
      checkbox.checked = task.available && state.selectedIds.has(task.id);
      checkbox.disabled = state.isRunning || !task.available;
      checkbox.addEventListener("change", () => {
        if (checkbox.checked) {
          state.selectedIds.add(task.id);
        } else {
          state.selectedIds.delete(task.id);
        }
        updateToolbar();
      });
      checkWrap.appendChild(checkbox);

      // Content column
      const content = document.createElement("div");
      content.className = "task-content";

      const titleRow = document.createElement("div");
      titleRow.className = "task-title-row";
      const titleLabel = document.createElement("label");
      titleLabel.className = "task-title-label";
      titleLabel.htmlFor = checkbox.id;
      titleLabel.textContent = task.title;
      titleRow.appendChild(titleLabel);

      if (task.badge) {
        const badge = document.createElement("span");
        badge.className = `badge ${task.badge === "Homebrew" ? "badge-brew" : task.badge === "Winget" ? "badge-winget" : ""}`;
        badge.textContent = task.badge;
        titleRow.appendChild(badge);
      }

      if (task.requiresAdmin) {
        const adminBadge = document.createElement("span");
        adminBadge.className = "badge badge-admin";
        adminBadge.textContent = "Admin";
        titleRow.appendChild(adminBadge);
      }

      content.appendChild(titleRow);

      const desc = document.createElement("p");
      desc.className = "task-desc";
      desc.textContent = task.description;
      content.appendChild(desc);

      if (!task.available && task.unavailableReason) {
        const note = document.createElement("p");
        note.className = "task-unavailable-note";
        note.textContent = task.unavailableReason;
        content.appendChild(note);
      }

      if (task.options && task.options.length > 0 && task.available) {
        const optsWrap = document.createElement("div");
        optsWrap.className = "task-options";
        const currentOpts = state.optionValues.get(task.id) || {};

        for (const opt of task.options) {
          const optLabel = document.createElement("label");
          optLabel.className = "option-item";
          const optInput = document.createElement("input");
          optInput.type = "checkbox";
          optInput.checked = Boolean(currentOpts[opt.key]);
          optInput.disabled = state.isRunning;
          optInput.addEventListener("change", () => {
            const updated = { ...(state.optionValues.get(task.id) || {}), [opt.key]: optInput.checked };
            state.optionValues.set(task.id, updated);
          });

          const optText = document.createElement("div");
          optText.className = "option-text";
          const optTitle = document.createElement("span");
          optTitle.textContent = opt.label;
          const optHint = document.createElement("span");
          optHint.className = "option-hint";
          optHint.textContent = opt.description;
          optText.appendChild(optTitle);
          optText.appendChild(optHint);

          optLabel.appendChild(optInput);
          optLabel.appendChild(optText);
          optsWrap.appendChild(optLabel);
        }
        content.appendChild(optsWrap);
      }

      if (runtime.status !== "idle") {
        const resultDiv = document.createElement("div");
        resultDiv.className = "task-result";
        const msg = document.createElement("span");
        msg.className = `task-result-msg ${runtime.status}`;
        if (runtime.status === "running") {
          msg.textContent = "Running maintenance task...";
        } else if (runtime.status === "queued") {
          msg.textContent = "Waiting in queue...";
        } else if (runtime.result?.summary) {
          msg.textContent = runtime.result.summary;
        }
        resultDiv.appendChild(msg);

        if (runtime.result?.openSettingsHint) {
          const settingsBtn = document.createElement("button");
          settingsBtn.type = "button";
          settingsBtn.className = "btn btn-outline btn-xs";
          settingsBtn.textContent = "Open Software Update Settings";
          settingsBtn.addEventListener("click", () => {
            invoke?.("open_software_update_settings").catch((err) => {
              appendLogLine(`Failed to open Software Update Settings: ${err}`, "stderr");
            });
          });
          resultDiv.appendChild(settingsBtn);
        }

        content.appendChild(resultDiv);
      }

      // Actions column
      const actions = document.createElement("div");
      actions.className = "task-actions";

      const chip = document.createElement("span");
      chip.className = `status-chip ${runtime.status}`;
      chip.textContent = statusLabel(runtime.status);

      const runBtn = document.createElement("button");
      runBtn.type = "button";
      runBtn.className = "btn btn-ghost btn-sm";
      runBtn.textContent = "Run";
      runBtn.disabled = state.isRunning || !task.available;
      runBtn.addEventListener("click", () => executeTasks([task]));

      actions.appendChild(chip);
      actions.appendChild(runBtn);

      card.appendChild(checkWrap);
      card.appendChild(content);
      card.appendChild(actions);
      fragment.appendChild(card);
    }

    els.taskList.innerHTML = "";
    els.taskList.appendChild(fragment);
    updateToolbar();
  }

  /* Task Execution */
  async function executeTasks(tasksToRun) {
    if (state.isRunning || !tasksToRun.length || !invoke) return;

    // Collect any destructive option confirmations before running
    const destructiveLabels = [];
    for (const task of tasksToRun) {
      const currentOpts = state.optionValues.get(task.id) || {};
      for (const opt of task.options || []) {
        if (opt.requiresConfirm && currentOpts[opt.key]) {
          destructiveLabels.push(opt.label);
        }
      }
    }

    if (destructiveLabels.length > 0) {
      const ok = await confirmDestructiveAction(destructiveLabels);
      if (!ok) return;
    }

    state.isRunning = true;
    state.cancelRequested = false;
    els.summaryBanner.classList.add("hidden");
    els.summarySettingsBtn.classList.add("hidden");
    els.reportIssueBtn.classList.add("hidden");

    for (const task of tasksToRun) {
      state.taskStates.set(task.id, { status: "queued", result: null });
    }
    renderTasks();

    const results = [];
    let shouldRefreshDisk = false;

    for (const task of tasksToRun) {
      if (state.cancelRequested) {
        state.taskStates.set(task.id, {
          status: "canceled",
          result: { summary: "Skipped after cancellation." },
        });
        results.push({ status: "canceled", bytesFreed: 0, rebootRequired: false, openSettingsHint: false });
        continue;
      }

      state.taskStates.set(task.id, { status: "running", result: null });
      renderTasks();
      appendLogLine(`=== ${task.title} ===`, "info");

      if (task.category === "cleanup" || task.category === "packages") {
        shouldRefreshDisk = true;
      }

      const opts = state.optionValues.get(task.id) || {};
      try {
        const res = await invoke("run_task", { taskId: task.id, options: opts });
        state.taskStates.set(task.id, { status: res.status, result: res });
        results.push(res);
        if (res.status === "canceled") {
          state.cancelRequested = true;
        }
      } catch (err) {
        const failRes = {
          taskId: task.id,
          status: "failed",
          summary: String(err),
          bytesFreed: null,
          rebootRequired: false,
          openSettingsHint: false,
        };
        state.taskStates.set(task.id, { status: "failed", result: failRes });
        results.push(failRes);
        appendLogLine(`Task error: ${err}`, "stderr");
      }
      renderTasks();
    }

    state.isRunning = false;
    state.cancelRequested = false;
    renderTasks();

    if (shouldRefreshDisk) {
      await refreshSystemInfo();
    }

    renderRunSummary(results);
  }

  function renderRunSummary(results) {
    if (!results.length) return;
    const total = results.length;
    const succeeded = results.filter((r) => r.status === "success").length;
    const warnings = results.filter((r) => r.status === "warning").length;
    const failed = results.filter((r) => r.status === "failed").length;
    const canceled = results.filter((r) => r.status === "canceled").length;
    const totalFreed = results.reduce((sum, r) => sum + (r.bytesFreed || 0), 0);
    const rebootNeeded = results.some((r) => r.rebootRequired);
    const settingsHint = results.some((r) => r.openSettingsHint);

    els.summaryBanner.className = "banner";
    if (failed > 0) {
      els.summaryBanner.classList.add("banner-error");
      els.summaryTitle.textContent = `Finished with ${failed} failed ${failed === 1 ? "task" : "tasks"}`;
    } else if (warnings > 0) {
      els.summaryBanner.classList.add("banner-warning");
      els.summaryTitle.textContent = "Completed with warnings";
    } else if (canceled > 0 && succeeded === 0) {
      els.summaryBanner.classList.add("banner-info");
      els.summaryTitle.textContent = "Maintenance canceled";
    } else {
      els.summaryBanner.classList.add("banner-success");
      els.summaryTitle.textContent = "Maintenance complete";
    }

    const parts = [`${succeeded} of ${total} succeeded`];
    if (warnings > 0) parts.push(`${warnings} warning`);
    if (failed > 0) parts.push(`${failed} failed`);
    if (canceled > 0) parts.push(`${canceled} canceled`);
    if (totalFreed > 0) parts.push(`Freed ${formatBytes(totalFreed)}`);
    if (rebootNeeded) parts.push("System restart recommended");

    els.summaryText.textContent = parts.join(" · ");
    els.summarySettingsBtn.classList.toggle("hidden", !settingsHint);
    els.reportIssueBtn.classList.toggle("hidden", failed === 0);
  }

  els.summarySettingsBtn.addEventListener("click", () => {
    invoke?.("open_software_update_settings").catch((err) => {
      appendLogLine(`Failed to open Software Update Settings: ${err}`, "stderr");
    });
  });

  els.reportIssueBtn.addEventListener("click", async () => {
    if (!invoke) return;
    try {
      const logPath = await invoke("report_error");
      appendLogLine(`Saved diagnostic log to ${logPath}`, "info");
    } catch (err) {
      appendLogLine(`Failed to save diagnostic report: ${err}`, "stderr");
    }
  });

  els.selectAllBtn.addEventListener("click", () => {
    for (const task of state.tasks) {
      if (task.available) state.selectedIds.add(task.id);
    }
    renderTasks();
  });

  els.selectNoneBtn.addEventListener("click", () => {
    state.selectedIds.clear();
    renderTasks();
  });

  els.runSelectedBtn.addEventListener("click", () => {
    const selectedTasks = state.tasks.filter((t) => t.available && state.selectedIds.has(t.id));
    executeTasks(selectedTasks);
  });

  els.cancelBtn.addEventListener("click", async () => {
    if (!state.isRunning || !invoke) return;
    state.cancelRequested = true;
    updateToolbar();
    appendLogLine("Canceling active task...", "info");
    try {
      await invoke("cancel_task");
    } catch (err) {
      appendLogLine(`Failed to cancel task: ${err}`, "stderr");
    }
  });

  /* Bootstrap */
  async function init() {
    syncThemeMetaFromStorage();

    if (!invoke || !listen) {
      els.taskList.innerHTML = '<div class="loading-state">Tauri runtime API was not detected.</div>';
      return;
    }

    await listen("task-output", (event) => {
      const payload = event.payload;
      if (payload?.line) {
        appendLogLine(payload.line, payload.stream || "stdout");
      }
    });

    try {
      const [info, tasks] = await Promise.all([invoke("get_system_info"), invoke("get_tasks")]);
      renderSystemInfo(info);
      state.tasks = tasks;

      for (const task of tasks) {
        if (task.available && task.defaultSelected) {
          state.selectedIds.add(task.id);
        }
        const defaults = {};
        for (const opt of task.options || []) {
          defaults[opt.key] = Boolean(opt.defaultValue);
        }
        state.optionValues.set(task.id, defaults);
        state.taskStates.set(task.id, { status: "idle", result: null });
      }

      renderTasks();
    } catch (err) {
      els.taskList.innerHTML = `<div class="loading-state">Failed to load system details: ${err}</div>`;
    }

    checkUpdatesNonBlocking();
  }

  init();
})();
