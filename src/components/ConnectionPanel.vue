<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore } from "../stores/modbus";

const store = useModbusStore();

const statusLabel = computed(() => (store.status.running ? "运行中" : "已停止"));
const bindLabel = computed(() =>
  store.status.bind ? store.status.bind : `${store.config.host}:${store.config.port}`
);
</script>

<template>
  <div class="connection-grid">
    <div class="field">
      <label>Host / 监听地址</label>
      <input v-model="store.config.host" type="text" placeholder="0.0.0.0" />
    </div>
    <div class="field">
      <label>Port / 端口</label>
      <input v-model.number="store.config.port" type="number" min="1" max="65535" />
    </div>
    <div class="field">
      <label>Unit ID / 站号</label>
      <input v-model.number="store.config.unit_id" type="number" min="0" max="247" />
    </div>
    <div class="field">
      <label>Bind / 绑定</label>
      <input :value="bindLabel" type="text" disabled />
    </div>
    <div class="field">
      <label>Status / 状态</label>
      <div class="status-chip" :class="{ running: store.status.running }">
        {{ statusLabel }}
      </div>
    </div>
    <div class="toolbar-actions">
      <button class="primary" :disabled="store.status.running" @click="store.startServer">
        Start / 启动
      </button>
      <button class="secondary" :disabled="!store.status.running" @click="store.stopServer">
        Stop / 停止
      </button>
    </div>
  </div>
</template>
