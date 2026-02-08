<script setup lang="ts">
import { ref, watch } from "vue";
import { useModbusStore } from "../stores/modbus";
import { useI18n } from "../lib/i18n";

const store = useModbusStore();
const { t } = useI18n();
const startAddress = ref(store.startAddress);
const pageSize = ref(store.pageSize);

watch(
  () => store.startAddress,
  (value) => {
    startAddress.value = value;
  }
);

watch(
  () => store.pageSize,
  (value) => {
    pageSize.value = value;
  }
);

function applyRange() {
  store.applyRange(startAddress.value, pageSize.value);
}
</script>

<template>
  <div class="panel navigation-panel">
    <div class="panel-title">{{ t("navigation.title") }}</div>
    <div class="field">
      <label>{{ t("navigation.startAddress") }}</label>
      <input v-model.number="startAddress" type="number" min="0" />
    </div>
    <div class="field">
      <label>{{ t("navigation.displayLength") }}</label>
      <input v-model.number="pageSize" type="number" min="1" max="100" />
    </div>
    <div class="side-actions">
      <button class="secondary" @click="store.prevPage">{{ t("navigation.prev") }}</button>
      <button class="secondary" @click="store.nextPage">{{ t("navigation.next") }}</button>
    </div>
    <div class="side-actions">
      <button class="primary" @click="applyRange">{{ t("navigation.apply") }}</button>
      <button class="secondary" @click="store.fetchSnapshot">{{ t("navigation.refresh") }}</button>
    </div>
    <div class="field">
      <label>{{ t("navigation.tips") }}</label>
      <div style="font-size: 12px; color: var(--text-muted);">
        {{ t("navigation.portTip") }}
      </div>
    </div>
  </div>
</template>
