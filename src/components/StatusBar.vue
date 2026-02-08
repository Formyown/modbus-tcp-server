<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore } from "../stores/modbus";
import { useI18n } from "../lib/i18n";

const store = useModbusStore();
const { t } = useI18n();

const runningLabel = computed(() =>
  store.status.running ? t("status.running") : t("status.stopped")
);
const bindLabel = computed(() => store.status.bind || t("status.unbound"));
</script>

<template>
  <div class="status-bar">
    <div class="status-indicator">
      <div class="status-dot" :class="{ running: store.status.running }"></div>
      <span>{{ runningLabel }}</span>
    </div>
    <div>{{ t("statusBar.bind") }}: {{ bindLabel }}</div>
    <div>{{ t("statusBar.connections") }}: {{ store.status.connections }}</div>
    <div v-if="store.status.last_error" class="status-error">
      {{ store.status.last_error }}
    </div>
  </div>
</template>
