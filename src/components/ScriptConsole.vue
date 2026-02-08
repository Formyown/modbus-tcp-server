<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, watch as watchFs, writeTextFile } from "@tauri-apps/plugin-fs";
import exampleContent from "../assets/examples/modbus_script_example.js?raw";
import { monaco } from "../lib/monaco";

type ConsoleTab = "script" | "logs";

type DataArea = "coils" | "discrete" | "input" | "holding";

interface UpdatePayload {
  area: DataArea;
  offset: number;
  values: number[];
}

type WatchStopHandle = () => void;
type ScriptCleanup = () => void;

const activeTab = ref<ConsoleTab>("script");
const collapsed = ref(false);
const PANEL_HEIGHT_STORAGE_KEY = "modbus.consolePanelHeight";
const DEFAULT_PANEL_HEIGHT = 260;
const MIN_PANEL_HEIGHT = 180;
const FALLBACK_WORKSPACE_HEIGHT = 180;
const COLLAPSED_HEIGHT = 48;
const MAX_LOG_LINES = 1000;
const panelHeight = ref(DEFAULT_PANEL_HEIGHT);
const isResizing = ref(false);
const editorHost = ref<HTMLElement | null>(null);
const panelHost = ref<HTMLElement | null>(null);
const logBox = ref<HTMLElement | null>(null);
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null);
const scriptContent = ref(exampleContent);
const lastSavedContent = ref(exampleContent);
const currentFilePath = ref<string | null>(null);
const dirty = ref(false);
const logs = ref<string[]>([]);
const externalChangePending = ref(false);
const externalDiskContent = ref<string | null>(null);
const isSettingValue = ref(false);
const isRunning = ref(false);
const followLogs = ref(true);

const scriptCleanups: ScriptCleanup[] = [];
let stopResolver: (() => void) | null = null;
let stopRequested = false;
let stopController: AbortController | null = null;

let stopWatch: WatchStopHandle | null = null;
let resizeObserver: ResizeObserver | null = null;
const resizeState = {
  startY: 0,
  startHeight: DEFAULT_PANEL_HEIGHT,
};

const collapseLabel = computed(() => (collapsed.value ? "Expand / 展开" : "Collapse / 收起"));
const panelStyle = computed(() => {
  const height = collapsed.value ? COLLAPSED_HEIGHT : panelHeight.value;
  return { height: `${height}px` };
});
const fileLabel = computed(() => {
  const name = currentFilePath.value ? basename(currentFilePath.value) : "example.js";
  return dirty.value ? `${name} •` : name;
});
const filePathLabel = computed(() => currentFilePath.value ?? "Unsaved example (use Save As)");

watch([collapsed, activeTab], async () => {
  await nextTick();
  layoutEditor();
});

function basename(path: string) {
  const parts = path.split(/[/\\]/g);
  return parts[parts.length - 1] || path;
}

function appendLog(message: string) {
  const stamp = new Date().toLocaleTimeString();
  const next = [...logs.value, `[${stamp}] ${message}`];
  if (next.length > MAX_LOG_LINES) {
    next.splice(0, next.length - MAX_LOG_LINES);
  }
  logs.value = next;
  scheduleScrollToBottom();
}

function scheduleScrollToBottom() {
  if (!followLogs.value) {
    return;
  }
  void nextTick(() => {
    const host = logBox.value;
    if (!host) {
      return;
    }
    host.scrollTop = host.scrollHeight;
  });
}

function toggleFollowLogs() {
  followLogs.value = !followLogs.value;
  if (followLogs.value) {
    scheduleScrollToBottom();
  }
}

async function saveLogs() {
  if (!logs.value.length) {
    appendLog("No logs to save.");
    return;
  }
  const selected = await save({
    defaultPath: "modbus-logs.txt",
    filters: [{ name: "Logs", extensions: ["txt", "log"] }],
  });
  if (!selected) {
    return;
  }
  try {
    const content = `${logs.value.join("\n")}\n`;
    await writeTextFile(selected, content);
    appendLog(`Saved logs to ${selected}`);
  } catch (error) {
    console.error(error);
    appendLog("Failed to save logs.");
  }
}

function registerCleanup(cleanup: ScriptCleanup) {
  scriptCleanups.push(cleanup);
}

function unregisterCleanup(cleanup: ScriptCleanup) {
  const index = scriptCleanups.indexOf(cleanup);
  if (index >= 0) {
    scriptCleanups.splice(index, 1);
  }
}

function clearScriptCleanups() {
  const cleanups = scriptCleanups.splice(0, scriptCleanups.length);
  cleanups.forEach((cleanup) => cleanup());
}

function resolveStop() {
  if (stopResolver) {
    stopResolver();
    stopResolver = null;
  }
}

function createStopError() {
  const error = new Error("Script stopped");
  error.name = "ScriptStopError";
  return error;
}

function isStopError(error: unknown) {
  return error instanceof Error && error.name === "ScriptStopError";
}

function formatLogArg(value: unknown) {
  if (typeof value === "string") {
    return value;
  }
  try {
    return JSON.stringify(value);
  } catch (error) {
    console.warn("Failed to stringify log value", error);
    return String(value);
  }
}

function logFromScript(...args: unknown[]) {
  appendLog(args.map(formatLogArg).join(" "));
}

function runSafeCallback(handler: (...args: unknown[]) => unknown, context: string, args: unknown[]) {
  try {
    const result = handler(...args);
    if (result && typeof (result as Promise<unknown>).then === "function") {
      void (result as Promise<unknown>).catch((error) => {
        console.error(`${context} failed`, error);
        appendLog(`${context} error: ${String(error)}`);
      });
    }
  } catch (error) {
    console.error(`${context} failed`, error);
    appendLog(`${context} error: ${String(error)}`);
  }
}

function normalizeDelay(value: unknown) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return 0;
  }
  return Math.max(0, numeric);
}

function scriptSetTimeout(handler: (...args: unknown[]) => unknown, ms?: number, ...args: unknown[]) {
  const delay = normalizeDelay(ms);
  let id = 0;
  const cleanup: ScriptCleanup = () => {
    window.clearTimeout(id);
  };
  id = window.setTimeout(() => {
    unregisterCleanup(cleanup);
    runSafeCallback(handler, "setTimeout", args);
  }, delay);
  registerCleanup(cleanup);
  return id;
}

function scriptSetInterval(handler: (...args: unknown[]) => unknown, ms?: number, ...args: unknown[]) {
  const delay = normalizeDelay(ms);
  let id = 0;
  const cleanup: ScriptCleanup = () => {
    window.clearInterval(id);
  };
  id = window.setInterval(() => {
    runSafeCallback(handler, "setInterval", args);
  }, delay);
  registerCleanup(cleanup);
  return id;
}

function sleep(ms: number) {
  const delay = Number(ms);
  return new Promise<void>((resolve, reject) => {
    const safeDelay = Number.isFinite(delay) ? Math.max(0, delay) : 0;
    let finished = false;
    const signal = stopController?.signal ?? null;
    let abortListener: (() => void) | null = null;
    const finish = () => {
      if (finished) {
        return;
      }
      finished = true;
      if (signal && abortListener) {
        signal.removeEventListener("abort", abortListener);
      }
      resolve();
    };
    const onAbort = () => {
      if (finished) {
        return;
      }
      finished = true;
      if (signal && abortListener) {
        signal.removeEventListener("abort", abortListener);
      }
      reject(createStopError());
    };

    const timerId = window.setTimeout(finish, safeDelay);
    if (signal) {
      if (signal.aborted) {
        window.clearTimeout(timerId);
        onAbort();
        return;
      }
      abortListener = () => {
        window.clearTimeout(timerId);
        onAbort();
      };
      signal.addEventListener("abort", abortListener, { once: true });
    }
  });
}

function normalizeRegisterValue(value: unknown) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return 0;
  }
  return Math.min(65535, Math.max(0, Math.round(numeric)));
}

async function writeAreaValues(
  area: DataArea,
  offset: number,
  valueOrValues: unknown | unknown[],
  isBitArea: boolean
) {
  if (Array.isArray(valueOrValues)) {
    if (isBitArea) {
      const payload = valueOrValues.map((value) => Boolean(value));
      await invoke("register_set_range", {
        area,
        offset,
        values: payload,
      });
      return;
    }
    const payload = valueOrValues.map((value) => normalizeRegisterValue(value));
    await invoke("register_set_range", {
      area,
      offset,
      values: payload,
    });
    return;
  }

  if (isBitArea) {
    await invoke("register_set", {
      area,
      offset,
      value: Boolean(valueOrValues),
    });
    return;
  }
  await invoke("register_set", {
    area,
    offset,
    value: normalizeRegisterValue(valueOrValues),
  });
}

async function writeCoils(offset: number, valueOrValues: unknown | unknown[]) {
  await writeAreaValues("coils", offset, valueOrValues, true);
}

async function writeDiscreteInputs(offset: number, valueOrValues: unknown | unknown[]) {
  await writeAreaValues("discrete", offset, valueOrValues, true);
}

async function writeHoldingRegs(offset: number, valueOrValues: unknown | unknown[]) {
  await writeAreaValues("holding", offset, valueOrValues, false);
}

async function writeInputRegs(offset: number, valueOrValues: unknown | unknown[]) {
  await writeAreaValues("input", offset, valueOrValues, false);
}

function onChange(handler: (payload: UpdatePayload) => void) {
  let disposed = false;
  let unlisten: (() => void) | null = null;

  const stop: ScriptCleanup = () => {
    if (disposed) {
      return;
    }
    disposed = true;
    if (unlisten) {
      unlisten();
    }
  };

  registerCleanup(stop);
  void listen<UpdatePayload>("modbus://updated", (event) => {
    runSafeCallback(handler, "onChange", [event.payload]);
  })
    .then((unlistenFn) => {
      if (disposed) {
        unlistenFn();
        return;
      }
      unlisten = unlistenFn;
    })
    .catch((error) => {
      console.error("Failed to listen for updates", error);
      appendLog("Failed to register onChange listener.");
    });

  return () => {
    stop();
    unregisterCleanup(stop);
  };
}

function stopScript() {
  if (!isRunning.value) {
    return;
  }
  appendLog("Stop requested.");
  stopRequested = true;
  stopController?.abort();
  resolveStop();
  clearScriptCleanups();
}

async function runScript() {
  if (isRunning.value) {
    appendLog("Script is already running.");
    return;
  }

  stopRequested = false;
  clearScriptCleanups();
  const content = editor.value ? editor.value.getValue() : scriptContent.value;
  scriptContent.value = content;
  stopController = new AbortController();
  const stopPromise = new Promise<void>((resolve) => {
    stopResolver = resolve;
  });

  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor as new (
    ...args: string[]
  ) => (...args: unknown[]) => Promise<unknown>;
  const runner = new AsyncFunction(
    "writeCoils",
    "writeDiscreteInputs",
    "writeHoldingRegs",
    "writeInputRegs",
    "onChange",
    "log",
    "setTimeout",
    "setInterval",
    "sleep",
    content
  );

  isRunning.value = true;
  appendLog("Script started.");
  let keepAlive = true;
  try {
    await runner(
      writeCoils,
      writeDiscreteInputs,
      writeHoldingRegs,
      writeInputRegs,
      onChange,
      logFromScript,
      scriptSetTimeout,
      scriptSetInterval,
      sleep
    );
    appendLog("Script finished.");
  } catch (error) {
    if (isStopError(error) || stopRequested) {
      appendLog("Script stopped.");
    } else {
      keepAlive = false;
      console.error("Script execution failed", error);
      appendLog(`Script error: ${String(error)}`);
    }
  } finally {
    if (keepAlive && !stopRequested) {
      appendLog("Script idle. Waiting for stop.");
      try {
        await stopPromise;
      } catch (error) {
        if (!isStopError(error)) {
          console.warn("Stop wait failed", error);
        }
      }
    }
    resolveStop();
    stopController = null;
    isRunning.value = false;
    stopRequested = false;
    clearScriptCleanups();
  }
}

function setTab(tab: ConsoleTab) {
  activeTab.value = tab;
  if (collapsed.value) {
    collapsed.value = false;
  }
}

function toggleCollapse() {
  collapsed.value = !collapsed.value;
  if (!collapsed.value) {
    ensurePanelHeight();
  }
}

function clearLogs() {
  logs.value = [];
}

function layoutEditor() {
  if (editor.value) {
    editor.value.layout();
  }
}

function loadPanelHeight() {
  if (typeof window === "undefined") {
    return;
  }
  const stored = window.localStorage.getItem(PANEL_HEIGHT_STORAGE_KEY);
  if (!stored) {
    return;
  }
  const parsed = Number.parseFloat(stored);
  if (!Number.isFinite(parsed)) {
    return;
  }
  panelHeight.value = parsed;
  ensurePanelHeight();
}

function savePanelHeight(value = panelHeight.value) {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(PANEL_HEIGHT_STORAGE_KEY, `${Math.round(value)}`);
}

function ensurePanelHeight() {
  panelHeight.value = clampPanelHeight(panelHeight.value);
}

function getMaxPanelHeight() {
  if (typeof window === "undefined") {
    return MIN_PANEL_HEIGHT;
  }
  const host = panelHost.value;
  const appShell = host?.parentElement;
  if (!appShell) {
    return MIN_PANEL_HEIGHT;
  }
  const shellRect = appShell.getBoundingClientRect();
  const toolbar = appShell.querySelector(".toolbar") as HTMLElement | null;
  const toolbarHeight = toolbar?.getBoundingClientRect().height ?? 0;
  const navigationMinHeight = getNavigationMinHeight(appShell);
  const style = window.getComputedStyle(appShell);
  const gapValue = Number.parseFloat(style.rowGap || style.gap || "0") || 0;
  const paddingTop = Number.parseFloat(style.paddingTop || "0") || 0;
  const paddingBottom = Number.parseFloat(style.paddingBottom || "0") || 0;
  const reserved = toolbarHeight + navigationMinHeight + gapValue * 2 + paddingTop + paddingBottom;
  return Math.max(MIN_PANEL_HEIGHT, Math.floor(shellRect.height - reserved));
}

function getNavigationMinHeight(appShell: HTMLElement) {
  const navigationPanel = appShell.querySelector(".navigation-panel") as HTMLElement | null;
  if (!navigationPanel) {
    return FALLBACK_WORKSPACE_HEIGHT;
  }
  const styles = window.getComputedStyle(navigationPanel);
  const paddingTop = Number.parseFloat(styles.paddingTop || "0") || 0;
  const paddingBottom = Number.parseFloat(styles.paddingBottom || "0") || 0;
  const borderTop = Number.parseFloat(styles.borderTopWidth || "0") || 0;
  const borderBottom = Number.parseFloat(styles.borderBottomWidth || "0") || 0;
  const gapValue = Number.parseFloat(styles.rowGap || styles.gap || "0") || 0;
  const children = Array.from(navigationPanel.children) as HTMLElement[];
  const childrenHeight = children.reduce((total, child) => {
    const rect = child.getBoundingClientRect();
    return total + rect.height;
  }, 0);
  const totalGaps = gapValue * Math.max(0, children.length - 1);
  const contentHeight = childrenHeight + totalGaps + paddingTop + paddingBottom + borderTop + borderBottom;
  const minHeight = Number.parseFloat(styles.minHeight || "0") || 0;
  return Math.max(FALLBACK_WORKSPACE_HEIGHT, Math.ceil(Math.max(contentHeight, minHeight)));
}

function clampPanelHeight(value: number) {
  const maxHeight = getMaxPanelHeight();
  return Math.min(Math.max(value, MIN_PANEL_HEIGHT), maxHeight);
}

function startResize(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  if (collapsed.value) {
    collapsed.value = false;
  }
  ensurePanelHeight();
  isResizing.value = true;
  resizeState.startY = event.clientY;
  resizeState.startHeight = panelHeight.value;
  (event.currentTarget as HTMLElement | null)?.setPointerCapture(event.pointerId);
  document.body.style.cursor = "row-resize";
  document.body.style.userSelect = "none";
  window.addEventListener("pointermove", handleResize);
  window.addEventListener("pointerup", stopResize, { once: true });
  window.addEventListener("pointercancel", stopResize, { once: true });
}

function handleResize(event: PointerEvent) {
  if (!isResizing.value) {
    return;
  }
  const delta = resizeState.startY - event.clientY;
  const nextHeight = resizeState.startHeight + delta;
  const clamped = clampPanelHeight(nextHeight);
  panelHeight.value = clamped;
  if (clamped !== nextHeight) {
    resizeState.startY = event.clientY;
    resizeState.startHeight = clamped;
  }
}

function stopResize() {
  if (!isResizing.value) {
    return;
  }
  isResizing.value = false;
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
  window.removeEventListener("pointermove", handleResize);
  savePanelHeight();
}

function handleWindowResize() {
  if (collapsed.value) {
    return;
  }
  ensurePanelHeight();
}

function updateEditorValue(value: string, markSaved = false) {
  if (!editor.value) {
    scriptContent.value = value;
    if (markSaved) {
      lastSavedContent.value = value;
      dirty.value = false;
    }
    return;
  }

  isSettingValue.value = true;
  editor.value.setValue(value);
  scriptContent.value = value;
  if (markSaved) {
    lastSavedContent.value = value;
    dirty.value = false;
  } else {
    dirty.value = value !== lastSavedContent.value;
  }
  isSettingValue.value = false;
}

async function confirmDiscard() {
  if (!dirty.value) {
    return true;
  }
  try {
    return await confirm("Discard unsaved changes?", {
      title: "Unsaved changes",
      kind: "warning",
    });
  } catch (error) {
    console.error(error);
    return window.confirm("Discard unsaved changes?");
  }
}

async function stopFileWatch() {
  if (stopWatch) {
    stopWatch();
    stopWatch = null;
  }
}

async function startFileWatch(path: string) {
  await stopFileWatch();
  try {
    stopWatch = await watchFs(
      path,
      () => {
        void handleExternalChange();
      },
      { recursive: false }
    );
  } catch (error) {
    console.error(error);
    appendLog("Watch failed.");
  }
}

async function handleExternalChange() {
  if (!currentFilePath.value) {
    return;
  }
  try {
    const diskContent = await readTextFile(currentFilePath.value);
    if (diskContent === scriptContent.value) {
      externalChangePending.value = false;
      return;
    }
    if (externalDiskContent.value === diskContent && !externalChangePending.value) {
      return;
    }
    externalDiskContent.value = diskContent;
    externalChangePending.value = true;
    appendLog("Detected external change.");
  } catch (error) {
    console.error(error);
    appendLog("Failed to read file changes.");
  }
}

async function reloadFromDisk() {
  if (externalDiskContent.value == null) {
    return;
  }
  updateEditorValue(externalDiskContent.value, true);
  externalDiskContent.value = null;
  externalChangePending.value = false;
  appendLog("Reloaded from disk.");
}

function keepLocal() {
  externalChangePending.value = false;
  appendLog("Kept local changes.");
}

async function openScript() {
  if (!(await confirmDiscard())) {
    return;
  }
  const selected = await open({
    multiple: false,
    filters: [{ name: "Script", extensions: ["js"] }],
  });
  if (!selected || Array.isArray(selected)) {
    return;
  }

  try {
    const content = await readTextFile(selected);
    currentFilePath.value = selected;
    externalChangePending.value = false;
    externalDiskContent.value = null;
    updateEditorValue(content, true);
    await startFileWatch(selected);
    appendLog(`Opened ${selected}`);
  } catch (error) {
    console.error(error);
    appendLog("Failed to open file.");
  }
}

async function saveScript() {
  if (!currentFilePath.value) {
    await saveScriptAs();
    return;
  }
  try {
    await writeTextFile(currentFilePath.value, scriptContent.value);
    lastSavedContent.value = scriptContent.value;
    dirty.value = false;
    appendLog(`Saved ${currentFilePath.value}`);
  } catch (error) {
    console.error(error);
    appendLog("Failed to save file.");
  }
}

async function saveScriptAs() {
  const selected = await save({
    defaultPath: currentFilePath.value ?? undefined,
    filters: [{ name: "Script", extensions: ["js"] }],
  });
  if (!selected) {
    return;
  }
  try {
    await writeTextFile(selected, scriptContent.value);
    currentFilePath.value = selected;
    lastSavedContent.value = scriptContent.value;
    dirty.value = false;
    externalChangePending.value = false;
    externalDiskContent.value = null;
    await startFileWatch(selected);
    appendLog(`Saved ${selected}`);
  } catch (error) {
    console.error(error);
    appendLog("Failed to save file.");
  }
}

async function loadExample() {
  if (!(await confirmDiscard())) {
    return;
  }
  currentFilePath.value = null;
  externalChangePending.value = false;
  externalDiskContent.value = null;
  updateEditorValue(exampleContent, true);
  await stopFileWatch();
  appendLog("Loaded example template.");
}

onMounted(() => {
  loadPanelHeight();
  if (!editorHost.value) {
    return;
  }
  const instance = monaco.editor.create(editorHost.value, {
    value: scriptContent.value,
    language: "javascript",
    theme: "vs",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 12,
    fixedOverflowWidgets: true,
    suggestOnTriggerCharacters: true,
    quickSuggestions: { other: true, comments: false, strings: true },
    snippetSuggestions: "inline",
    tabCompletion: "on",
  });
  editor.value = instance;
  instance.onDidChangeModelContent(() => {
    if (isSettingValue.value) {
      return;
    }
    scriptContent.value = instance.getValue();
    dirty.value = scriptContent.value !== lastSavedContent.value;
  });

  if (panelHost.value) {
    resizeObserver = new ResizeObserver(() => layoutEditor());
    resizeObserver.observe(panelHost.value);
  }

  nextTick(() => {
    ensurePanelHeight();
    layoutEditor();
  });
  window.addEventListener("resize", handleWindowResize);
});

onBeforeUnmount(async () => {
  await stopFileWatch();
  clearScriptCleanups();
  resizeObserver?.disconnect();
  editor.value?.dispose();
  stopResize();
  window.removeEventListener("resize", handleWindowResize);
});
</script>

<template>
  <section
    ref="panelHost"
    class="console-panel"
    :class="{ collapsed, resizing: isResizing }"
    :style="panelStyle"
  >
    <div class="console-resizer" @pointerdown="startResize"></div>
    <header class="console-header">
      <div class="console-tabs">
        <button
          class="console-tab"
          :class="{ active: activeTab === 'script' }"
          @click="setTab('script')"
        >
          <svg class="tab-icon" viewBox="0 0 20 20" aria-hidden="true">
            <path d="M7.5 6.5l-3 3 3 3" />
            <path d="M12.5 6.5l3 3-3 3" />
            <path d="M9.5 6l1 8" />
          </svg>
          Script Editor / 脚本编辑
        </button>
        <button
          class="console-tab"
          :class="{ active: activeTab === 'logs' }"
          @click="setTab('logs')"
        >
          <svg class="tab-icon" viewBox="0 0 20 20" aria-hidden="true">
            <path d="M6 6.5h8" />
            <path d="M6 10h8" />
            <path d="M6 13.5h5" />
            <path d="M4 4.5h12v11H4z" />
          </svg>
          Logs / 运行日志
        </button>
      </div>
      <div class="console-actions">
        <span v-if="isRunning" class="console-running-chip">Running / 运行中</span>
        <button
          class="secondary collapse-toggle"
          @click="toggleCollapse"
          :aria-label="collapseLabel"
          :title="collapseLabel"
        >
          <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
            <path v-if="collapsed" d="M5 12l5-5 5 5" />
            <path v-else d="M5 8l5 5 5-5" />
          </svg>
          <span class="collapse-label">{{ collapseLabel }}</span>
        </button>
      </div>
    </header>

    <div v-show="!collapsed" class="console-body">
      <div v-show="activeTab === 'script'" class="console-pane">
        <div class="console-file-info">
          <div class="console-file-name">{{ fileLabel }}</div>
          <div class="console-file-path">{{ filePathLabel }}</div>
        </div>
        <div v-if="externalChangePending" class="console-banner">
          <span>File changed on disk / 文件已更新</span>
          <div class="console-banner-actions">
            <button class="secondary" @click="reloadFromDisk">Reload</button>
            <button class="secondary" @click="keepLocal">Keep</button>
          </div>
        </div>
        <div ref="editorHost" class="console-editor"></div>
        <div class="console-toolbar">
          <button class="secondary" @click="openScript" aria-label="Open" title="Open">
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M2.5 8h5l1.6 2h8.4v5.5a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2z" />
              <path d="M2.5 8V5.5a2 2 0 0 1 2-2h3l1.5 2" />
            </svg>
            <span>Open</span>
          </button>
          <button
            class="secondary"
            @click="saveScript"
            :disabled="!dirty"
            aria-label="Save"
            title="Save"
          >
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M5.5 3.5h8l3 3v9.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 4 15.5V5A1.5 1.5 0 0 1 5.5 3.5z" />
              <path d="M7 3.5v5h6v-5" />
              <path d="M7 13h6" />
            </svg>
            <span>Save</span>
          </button>
          <button class="secondary" @click="saveScriptAs" aria-label="Save As" title="Save As">
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M5.5 3.5h8l3 3v9.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 4 15.5V5A1.5 1.5 0 0 1 5.5 3.5z" />
              <path d="M7 3.5v5h6v-5" />
              <path d="M10 11v4" />
              <path d="M8 13h4" />
            </svg>
            <span>Save As</span>
          </button>
          <button class="secondary" @click="loadExample" aria-label="Example" title="Example">
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M6 3.5h6l4 4v8.5a1.5 1.5 0 0 1-1.5 1.5h-8A1.5 1.5 0 0 1 5 15.5V5A1.5 1.5 0 0 1 6 3.5z" />
              <path d="M12 3.5v4h4" />
              <path d="M8 11.5l-2 2 2 2" />
              <path d="M12 11.5l2 2-2 2" />
            </svg>
            <span>Example</span>
          </button>
          <button
            class="primary script-run"
            :class="{ danger: isRunning }"
            :aria-label="isRunning ? 'Stop / 停止' : 'Run / 运行'"
            :title="isRunning ? 'Stop / 停止' : 'Run / 运行'"
            @click="isRunning ? stopScript() : runScript()"
          >
            <svg class="button-icon solid" viewBox="0 0 20 20" aria-hidden="true">
              <path v-if="isRunning" d="M5 5h10v10H5z" />
              <path v-else d="M6 4.5l8.5 5-8.5 5z" />
            </svg>
            <span>{{ isRunning ? "Stop / 停止" : "Run / 运行" }}</span>
          </button>
        </div>
        <div class="console-hint">
          Tip: helpers: writeCoils / writeDiscreteInputs / writeHoldingRegs / writeInputRegs / onChange /
          log / setTimeout / setInterval / sleep.
        </div>
      </div>

      <div v-show="activeTab === 'logs'" class="console-pane">
        <div ref="logBox" class="console-log-box">
          <div v-if="!logs.length" class="console-empty">No logs yet.</div>
          <div v-else class="console-log-list">
            <div v-for="(line, index) in logs" :key="index" class="console-log-line">
              {{ line }}
            </div>
          </div>
        </div>
        <div class="console-toolbar">
          <button
            class="secondary icon-only"
            :class="{ active: followLogs }"
            @click="toggleFollowLogs"
            aria-label="Follow logs / 跟踪日志"
            title="Follow logs / 跟踪日志"
          >
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M10 3v10" />
              <path d="M6.5 9.5l3.5 3.5 3.5-3.5" />
              <path d="M4 16.5h12" />
            </svg>
          </button>
          <button
            class="secondary icon-only"
            @click="saveLogs"
            :disabled="!logs.length"
            aria-label="Save logs / 保存日志"
            title="Save logs / 保存日志"
          >
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M5.5 3.5h8l3 3v9.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 4 15.5V5A1.5 1.5 0 0 1 5.5 3.5z" />
              <path d="M7 3.5v5h6v-5" />
              <path d="M7 13h6" />
            </svg>
          </button>
          <button
            class="secondary"
            @click="clearLogs"
            :disabled="!logs.length"
            aria-label="Clear / 清空"
            title="Clear / 清空"
          >
            <svg class="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M4.5 6.5h11" />
              <path d="M6 6.5l1-2h6l1 2" />
              <path d="M7.5 6.5v8" />
              <path d="M12.5 6.5v8" />
              <path d="M5.5 6.5v8.5a1.5 1.5 0 0 0 1.5 1.5h6a1.5 1.5 0 0 0 1.5-1.5V6.5" />
            </svg>
            <span>Clear / 清空</span>
          </button>
        </div>
        <div class="console-hint">Logs will show script runs, register writes, and connection events.</div>
      </div>
    </div>
  </section>
</template>
