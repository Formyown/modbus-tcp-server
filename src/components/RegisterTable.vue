<script setup lang="ts">
import { computed } from "vue";
import { useModbusStore, type DataArea } from "../stores/modbus";
import { useI18n } from "../lib/i18n";

const store = useModbusStore();
const { t } = useI18n();

const rows = computed(() => store.rows);
const isBitArea = computed(() => store.isBitArea);
const onLabel = computed(() => t("registers.on"));
const offLabel = computed(() => t("registers.off"));

const ADDRESS_BASES: Record<DataArea, number> = {
  coils: 1,
  discrete: 10001,
  input: 30001,
  holding: 40001,
};

const addressBase = computed(() => ADDRESS_BASES[store.area]);

function formatAddress(offset: number) {
  const address = addressBase.value + offset;
  return String(address).padStart(5, "0");
}

function clampRegister(value: number) {
  if (Number.isNaN(value)) {
    return 0;
  }
  return Math.min(65535, Math.max(0, Math.round(value)));
}

function onToggle(address: number, nextValue: boolean) {
  store.setValue(address, nextValue);
}

function onNumberChange(address: number, event: Event) {
  const target = event.target as HTMLInputElement;
  const numeric = clampRegister(Number(target.value));
  target.value = String(numeric);
  store.setValue(address, numeric);
}
</script>

<template>
  <div class="panel table-panel">
    <div class="panel-title">{{ t("registers.title") }}</div>
    <div class="table-scroll">
      <table class="register-table">
        <thead>
          <tr>
            <th style="width: 140px;">{{ t("registers.modbusAddress") }}</th>
            <th>{{ t("registers.value") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in rows" :key="row.address">
            <td>{{ formatAddress(row.address) }}</td>
            <td v-if="isBitArea" class="bit-cell">
              <button
                type="button"
                class="bit-led"
                :class="{ on: row.value !== 0 }"
                :aria-pressed="row.value !== 0"
                :aria-label="row.value !== 0 ? onLabel : offLabel"
                @click="onToggle(row.address, row.value === 0)"
              />
            </td>
            <td v-else>
              <input
                type="number"
                :value="row.value"
                min="0"
                max="65535"
                step="1"
                @change="onNumberChange(row.address, $event)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
