<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import {
  getCurrentWindow,
  LogicalSize,
  type PhysicalSize,
  type Window as TauriWindow,
} from "@tauri-apps/api/window";
import ConnectionPanel from "./components/ConnectionPanel.vue";
import RegisterTabs from "./components/RegisterTabs.vue";
import RegisterTable from "./components/RegisterTable.vue";
import SidePanel from "./components/SidePanel.vue";
import ScriptConsole from "./components/ScriptConsole.vue";
import { useModbusStore } from "./stores/modbus";

const store = useModbusStore();
const WINDOW_SIZE_STORAGE_KEY = "modbus.windowSize";
const WINDOW_RESIZE_DEBOUNCE_MS = 200;
let resizeTimeout: number | null = null;
let unlistenResize: (() => void) | null = null;

onMounted(() => {
  store.initialize();
  void setupWindowState();
});

onBeforeUnmount(() => {
  if (resizeTimeout != null) {
    window.clearTimeout(resizeTimeout);
  }
  unlistenResize?.();
});

function loadWindowSize() {
  if (typeof window === "undefined") {
    return null;
  }
  const raw = window.localStorage.getItem(WINDOW_SIZE_STORAGE_KEY);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as { width: number; height: number };
    if (!Number.isFinite(parsed.width) || !Number.isFinite(parsed.height)) {
      return null;
    }
    return parsed;
  } catch (error) {
    console.warn("Failed to parse window size", error);
    return null;
  }
}

function saveWindowSize(size: { width: number; height: number }) {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(WINDOW_SIZE_STORAGE_KEY, JSON.stringify(size));
}

function scheduleWindowSizeSave(size: { width: number; height: number }) {
  if (typeof window === "undefined") {
    return;
  }
  if (resizeTimeout != null) {
    window.clearTimeout(resizeTimeout);
  }
  resizeTimeout = window.setTimeout(() => {
    saveWindowSize(size);
  }, WINDOW_RESIZE_DEBOUNCE_MS);
}

async function handleWindowResized(appWindow: TauriWindow, size: PhysicalSize) {
  try {
    const scaleFactor = await appWindow.scaleFactor();
    const logical = size.toLogical(scaleFactor);
    scheduleWindowSizeSave({
      width: Math.round(logical.width),
      height: Math.round(logical.height),
    });
  } catch (error) {
    console.warn("Failed to save window size", error);
  }
}

async function setupWindowState() {
  let appWindow: TauriWindow;
  try {
    appWindow = getCurrentWindow();
  } catch (error) {
    console.warn("Window state not available", error);
    return;
  }

  const stored = loadWindowSize();
  if (stored) {
    try {
      await appWindow.setSize(new LogicalSize(stored.width, stored.height));
    } catch (error) {
      console.warn("Failed to restore window size", error);
    }
  }

  try {
    unlistenResize = await appWindow.onResized(({ payload }) => {
      void handleWindowResized(appWindow, payload);
    });
  } catch (error) {
    console.warn("Failed to listen for window resize", error);
  }
}
</script>

<template>
  <div class="app-shell">
    <header class="toolbar">
      <div class="title-block">
        <h1>Modbus TCP Server</h1>
        <span>仿真 / 测试 / 编程 控制面板</span>
      </div>
      <ConnectionPanel />
    </header>

    <section class="workspace">
      <RegisterTabs />
      <RegisterTable />
      <SidePanel />
    </section>

    <ScriptConsole />
  </div>
</template>
