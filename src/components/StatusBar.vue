<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore } from "../stores/modbus";

const store = useModbusStore();

const runningLabel = computed(() => (store.status.running ? "Running / 运行中" : "Stopped / 已停止"));
const bindLabel = computed(() => store.status.bind || "未绑定");
</script>

<template>
  <div class="status-bar">
    <div class="status-indicator">
      <div class="status-dot" :class="{ running: store.status.running }"></div>
      <span>{{ runningLabel }}</span>
    </div>
    <div>Bind: {{ bindLabel }}</div>
    <div>Connections: {{ store.status.connections }}</div>
    <div v-if="store.status.last_error" class="status-error">
      {{ store.status.last_error }}
    </div>
  </div>
</template>
