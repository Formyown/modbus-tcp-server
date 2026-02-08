<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore, type DataArea } from "../stores/modbus";
import { useI18n } from "../lib/i18n";

const store = useModbusStore();
const { t } = useI18n();

const tabs = computed<Array<{ area: DataArea; label: string; hint: string }>>(() => [
  { area: "coils", label: t("dataAreas.coils"), hint: t("dataAreas.coilsHint") },
  { area: "discrete", label: t("dataAreas.discrete"), hint: t("dataAreas.discreteHint") },
  { area: "input", label: t("dataAreas.input"), hint: t("dataAreas.inputHint") },
  { area: "holding", label: t("dataAreas.holding"), hint: t("dataAreas.holdingHint") },
]);
</script>

<template>
  <div class="panel">
    <div class="panel-title">{{ t("dataAreas.title") }}</div>
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
