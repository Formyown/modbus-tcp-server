<script setup lang="ts">
import { useModbusStore, type DataArea } from "../stores/modbus";

const store = useModbusStore();

const tabs: Array<{ area: DataArea; label: string; hint: string }> = [
  { area: "coils", label: "Coils", hint: "线圈 / 可写" },
  { area: "discrete", label: "Discrete", hint: "离散输入" },
  { area: "input", label: "Input Reg", hint: "输入寄存器" },
  { area: "holding", label: "Holding Reg", hint: "保持寄存器" },
];
</script>

<template>
  <div class="panel">
    <div class="panel-title">Data Areas</div>
    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.area"
        class="tab-button"
        :class="{ active: store.area === tab.area }"
        @click="store.setArea(tab.area)"
      >
        <strong>{{ tab.label }}</strong>
        <span>{{ tab.hint }}</span>
      </button>
    </div>
  </div>
</template>
