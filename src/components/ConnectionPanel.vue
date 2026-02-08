<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore } from "../stores/modbus";
import { useI18n } from "../lib/i18n";

const store = useModbusStore();
const { t } = useI18n();

const statusLabel = computed(() =>
  store.status.running ? t("status.running") : t("status.stopped")
);
const bindLabel = computed(() =>
  store.status.bind ? store.status.bind : `${store.config.host}:${store.config.port}`
);
</script>

<template>
  <div class="connection-grid">
    <div class="field">
      <label>{{ t("connection.host") }}</label>
      <input v-model="store.config.host" type="text" placeholder="0.0.0.0" />
    </div>
    <div class="field">
      <label>{{ t("connection.port") }}</label>
      <input v-model.number="store.config.port" type="number" min="1" max="65535" />
    </div>
    <div class="field">
      <label>{{ t("connection.unitId") }}</label>
      <input v-model.number="store.config.unit_id" type="number" min="0" max="247" />
    </div>
    <div class="field">
      <label>{{ t("connection.bind") }}</label>
      <input :value="bindLabel" type="text" disabled />
    </div>
    <div class="field">
      <label>{{ t("connection.status") }}</label>
      <div class="status-chip" :class="{ running: store.status.running }">
        {{ statusLabel }}
      </div>
    </div>
    <div class="toolbar-actions">
      <button class="primary" :disabled="store.status.running" @click="store.startServer">
        {{ t("connection.start") }}
      </button>
      <button class="secondary" :disabled="!store.status.running" @click="store.stopServer">
        {{ t("connection.stop") }}
      </button>
    </div>
  </div>
</template>
