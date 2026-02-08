<script setup lang="ts">
import { ref, watch } from "vue";
import { useModbusStore } from "../stores/modbus";

const store = useModbusStore();
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
    <div class="panel-title">Navigation</div>
    <div class="field">
      <label>Start Address / 起始地址</label>
      <input v-model.number="startAddress" type="number" min="0" />
    </div>
    <div class="field">
      <label>Display Length / 显示长度</label>
      <input v-model.number="pageSize" type="number" min="1" max="100" />
    </div>
    <div class="side-actions">
      <button class="secondary" @click="store.prevPage">Prev / 上一页</button>
      <button class="secondary" @click="store.nextPage">Next / 下一页</button>
    </div>
    <div class="side-actions">
      <button class="primary" @click="applyRange">Apply / 跳转</button>
      <button class="secondary" @click="store.fetchSnapshot">Refresh / 刷新</button>
    </div>
    <div class="field">
      <label>Tips / 提示</label>
      <div style="font-size: 12px; color: var(--text-muted);">
        端口 502 需要管理员权限，建议使用 1502 进行测试。
      </div>
    </div>
  </div>
</template>
